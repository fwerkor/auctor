#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CONFIG = Path(os.environ.get("INCUS_AUCTOR_RBAC_CONFIG", "/etc/incus/auctor-rbac.json"))
APP_CLIENT_ID = "fwerkor-incus"
ADMIN_ROLE = "platform-admin"
ROLES = ("viewer", "user", "operator", "admin")
ROLE_ORDER = {name: i for i, name in enumerate(ROLES)}


def run(cmd, *, input_text=None, check=True):
    return subprocess.run(
        cmd,
        input=input_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=check,
    )


def psql(sql):
    result = run([
        "incus", "exec", "auctor", "--",
        "sudo", "-u", "postgres", "psql", "-d", "auctor",
        "-q", "-A", "-F", "|", "-t", "-c", sql,
    ])
    return [line for line in result.stdout.splitlines() if line.strip()]


def sql_literal(value):
    return "'" + value.replace("'", "''") + "'"


def load_config():
    if not CONFIG.exists():
        return {"application": APP_CLIENT_ID, "grants": []}
    data = json.loads(CONFIG.read_text())
    if data.get("application", APP_CLIENT_ID) != APP_CLIENT_ID:
        raise SystemExit(f"unsupported application: {data.get('application')}")
    grants = data.get("grants", [])
    if not isinstance(grants, list):
        raise SystemExit("grants must be a list")
    for grant in grants:
        if not isinstance(grant, dict):
            raise SystemExit("each grant must be an object")
        if not all(isinstance(grant.get(k), str) and grant[k] for k in ("group", "project", "role")):
            raise SystemExit("grant requires non-empty group/project/role")
        if grant["role"] not in ROLES:
            raise SystemExit(f"invalid role {grant['role']!r}")
    return {"application": APP_CLIENT_ID, "grants": grants}


def save_config(data):
    CONFIG.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".auctor-rbac-", dir=str(CONFIG.parent), text=True)
    try:
        with os.fdopen(fd, "w") as f:
            json.dump(data, f, indent=2, sort_keys=True)
            f.write("\n")
        os.chmod(tmp, 0o600)
        os.replace(tmp, CONFIG)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def validate_group(group):
    rows = psql(f"SELECT name FROM groups WHERE name={sql_literal(group)};")
    if not rows:
        raise SystemExit(f"Auctor group does not exist: {group}")


def validate_project(project):
    result = run(["incus", "project", "show", project], check=False)
    if result.returncode != 0:
        raise SystemExit(f"Incus project does not exist: {project}")


def get_admins():
    return psql(
        "SELECT DISTINCT u.username "
        "FROM users u "
        "JOIN user_roles ur ON ur.user_id=u.id "
        "JOIN roles r ON r.id=ur.role_id "
        f"WHERE r.name={sql_literal(ADMIN_ROLE)} AND u.status='active' "
        "ORDER BY u.username;"
    )


def get_group_members():
    rows = psql(
        "SELECT g.name,u.username "
        "FROM groups g "
        "JOIN user_groups ug ON ug.group_id=g.id "
        "JOIN users u ON u.id=ug.user_id "
        "WHERE u.status='active' "
        "ORDER BY g.name,u.username;"
    )
    result = {}
    for line in rows:
        group, username = line.split("|", 1)
        result.setdefault(group, []).append(username)
    return result


def get_issuer():
    rows = psql("SELECT site_url FROM site_settings WHERE singleton=true;")
    if len(rows) != 1 or not rows[0].startswith("https://"):
        raise SystemExit("Auctor site_url must be one HTTPS issuer")
    return rows[0].rstrip("/")


def resolve_user_grants(config, members):
    resolved = {}
    for grant in config["grants"]:
        for username in members.get(grant["group"], []):
            user_grants = resolved.setdefault(username, {})
            old = user_grants.get(grant["project"])
            if old is None or ROLE_ORDER[grant["role"]] > ROLE_ORDER[old]:
                user_grants[grant["project"]] = grant["role"]
    return resolved


def render_scriptlet(admins, user_grants):
    # JSON string/list/dict syntax is also valid Starlark for these values.
    admins_value = json.dumps(sorted(set(admins)))
    grants_value = json.dumps(user_grants, sort_keys=True)
    return f'''# Managed by /usr/local/sbin/incus-auctor-rbac. Do not edit by hand.
ADMINS = {admins_value}
GRANTS = {grants_value}

SERVER_PUBLIC = ["can_view", "can_view_metrics", "can_view_resources"]
PROJECT_USER = ["can_view", "can_view_events", "can_view_operations"]
PROJECT_OPERATOR = PROJECT_USER + [
  "can_create_image_aliases", "can_create_images", "can_create_instances",
  "can_create_network_acls", "can_create_network_address_sets",
  "can_create_networks", "can_create_network_zones", "can_create_profiles",
  "can_create_storage_buckets", "can_create_storage_volumes",
]
INSTANCE_USER = ["can_view", "can_access_console", "can_access_files", "can_connect_sftp", "can_exec"]
INSTANCE_OPERATOR = INSTANCE_USER + ["can_edit", "can_manage_backups", "can_manage_snapshots", "can_update_state"]
VOLUME_OPERATOR = ["can_view", "can_edit", "can_access_files", "can_connect_sftp", "can_manage_backups", "can_manage_snapshots"]
PROJECT_SCOPED = [
  "image", "image_alias", "network", "network_acl", "network_address_set",
  "network_zone", "profile", "storage_bucket",
]

def authorize(details, object, entitlement):
  # Keep local unix/TLS administration unchanged. This policy is for Auctor OIDC identities.
  if details.Protocol != "oidc":
    return True

  username = details.Username
  if username in ADMINS:
    return True

  # Match Incus' authenticated server/storage-pool read permissions.
  if object == "server:incus":
    return entitlement in SERVER_PUBLIC
  if object.startswith("storage_pool:"):
    return entitlement == "can_view"

  grants = GRANTS.get(username, {{}})
  for project, role in grants.items():
    if object == "project:" + project:
      if role == "viewer":
        return entitlement == "can_view"
      if role == "user":
        return entitlement in PROJECT_USER
      if role == "operator":
        return entitlement in PROJECT_OPERATOR
      if role == "admin":
        return entitlement in PROJECT_OPERATOR or entitlement == "can_edit"

    if object.startswith("instance:" + project + "/"):
      if role == "viewer":
        return entitlement == "can_view"
      if role == "user":
        return entitlement in INSTANCE_USER
      return entitlement in INSTANCE_OPERATOR

    if object.startswith("storage_volume:" + project + "/"):
      if role in ["operator", "admin"]:
        return entitlement in VOLUME_OPERATOR
      return entitlement == "can_view"

    for object_type in PROJECT_SCOPED:
      if object.startswith(object_type + ":" + project + "/"):
        if role in ["operator", "admin"]:
          return entitlement in ["can_view", "can_edit"]
        return entitlement == "can_view"

  return False

def get_project_access(project_name):
  users = []
  for username, grants in GRANTS.items():
    if project_name in grants:
      users.append(username)
  return users
'''


def update_application_policy(config):
    allowed_groups = sorted({grant["group"] for grant in config["grants"]})
    roles_json = json.dumps([ADMIN_ROLE], separators=(",", ":"))
    groups_json = json.dumps(allowed_groups, separators=(",", ":"))
    sql = (
        "UPDATE applications SET "
        f"allowed_roles={sql_literal(roles_json)}::jsonb,"
        f"allowed_groups={sql_literal(groups_json)}::jsonb,"
        "updated_at=now() "
        f"WHERE client_id={sql_literal(APP_CLIENT_ID)} "
        "RETURNING client_id;"
    )
    rows = psql(sql)
    if APP_CLIENT_ID not in rows:
        raise SystemExit(f"Auctor application not found: {APP_CLIENT_ID}")


def incus_config_get(key):
    result = run(["incus", "config", "get", key], check=False)
    if result.returncode != 0:
        return None
    return result.stdout.rstrip("\n")


def incus_config_set_if_changed(key, value):
    if incus_config_get(key) != value.rstrip("\n"):
        run(["incus", "config", "set", key, value])


def sync():
    config = load_config()
    admins = get_admins()
    if not admins:
        raise SystemExit("refusing to sync: no active platform-admin users")
    members = get_group_members()
    user_grants = resolve_user_grants(config, members)

    # Ensure Auctor itself rejects identities that are neither admins nor in a granted group.
    update_application_policy(config)

    scriptlet = render_scriptlet(admins, user_grants)
    if incus_config_get("authorization.scriptlet") != scriptlet.rstrip("\n"):
        run(["incus", "config", "set", "authorization.scriptlet=-"], input_text=scriptlet)

    issuer = get_issuer()
    settings = {
        "oidc.issuer": issuer,
        "oidc.client.id": APP_CLIENT_ID,
        "oidc.audience": APP_CLIENT_ID,
        "oidc.claim": "preferred_username",
        "oidc.scopes": "openid,profile,email,groups,roles,offline_access",
        "user.ui.sso_only": "true",
    }
    for key, value in settings.items():
        incus_config_set_if_changed(key, value)

    print(json.dumps({
        "ok": True,
        "admins": admins,
        "configured_groups": sorted({g["group"] for g in config["grants"]}),
        "resolved_users": len(user_grants),
        "grants": config["grants"],
    }, ensure_ascii=False))


def show():
    config = load_config()
    members = get_group_members()
    print(json.dumps({
        "config": config,
        "admins": get_admins(),
        "resolved": resolve_user_grants(config, members),
    }, ensure_ascii=False, indent=2))


def grant(group, project, role):
    if role not in ROLES:
        raise SystemExit("role must be one of: " + ", ".join(ROLES))
    validate_group(group)
    validate_project(project)
    config = load_config()
    grants = [
        g for g in config["grants"]
        if not (g["group"] == group and g["project"] == project)
    ]
    grants.append({"group": group, "project": project, "role": role})
    config["grants"] = sorted(grants, key=lambda x: (x["group"], x["project"], x["role"]))
    save_config(config)
    sync()


def revoke(group, project):
    config = load_config()
    before = len(config["grants"])
    config["grants"] = [
        g for g in config["grants"]
        if not (g["group"] == group and g["project"] == project)
    ]
    if len(config["grants"]) == before:
        raise SystemExit(f"grant not found: {group} -> {project}")
    save_config(config)
    sync()


def usage():
    print(
        "usage:\n"
        "  incus-auctor-rbac sync\n"
        "  incus-auctor-rbac show\n"
        "  incus-auctor-rbac grant <auctor-group> <incus-project> <viewer|user|operator|admin>\n"
        "  incus-auctor-rbac revoke <auctor-group> <incus-project>",
        file=sys.stderr,
    )
    raise SystemExit(2)


def main():
    if os.geteuid() != 0:
        raise SystemExit("run as root")
    if len(sys.argv) < 2:
        usage()
    cmd = sys.argv[1]
    if cmd == "sync" and len(sys.argv) == 2:
        sync()
    elif cmd == "show" and len(sys.argv) == 2:
        show()
    elif cmd == "grant" and len(sys.argv) == 5:
        grant(sys.argv[2], sys.argv[3], sys.argv[4])
    elif cmd == "revoke" and len(sys.argv) == 4:
        revoke(sys.argv[2], sys.argv[3])
    else:
        usage()


if __name__ == "__main__":
    main()
