<?php
/**
 * Plugin Name: Auctor SSO
 * Description: Sign in to WordPress with an Auctor identity using OAuth 2.0 Authorization Code + PKCE.
 * Version: 0.1.0
 * Author: Auctor
 * License: Apache-2.0
 */

defined('ABSPATH') || exit;

final class Auctor_SSO {
    const OPTION = 'auctor_sso_options';
    const META_SUB = 'auctor_sub';
    const TRANSIENT_PREFIX = 'auctor_oauth_';

    public static function init(): void {
        add_action('init', [__CLASS__, 'handle_callback'], 1);
        add_action('login_form', [__CLASS__, 'render_login_button']);
        add_action('login_init', [__CLASS__, 'maybe_force_login']);
        add_action('admin_menu', [__CLASS__, 'admin_menu']);
        add_action('admin_init', [__CLASS__, 'register_settings']);
        add_shortcode('auctor_login_button', [__CLASS__, 'shortcode']);
    }

    public static function defaults(): array {
        return [
            'issuer' => '',
            'client_id' => '',
            'allow_registration' => 1,
            'force_login' => 0,
            'button_label' => 'Sign in with Auctor',
        ];
    }

    public static function options(): array {
        $value = get_option(self::OPTION, []);
        return array_merge(self::defaults(), is_array($value) ? $value : []);
    }

    public static function callback_url(): string {
        return home_url('/?auth=auctor');
    }

    public static function register_settings(): void {
        register_setting('auctor_sso', self::OPTION, [
            'type' => 'array',
            'sanitize_callback' => [__CLASS__, 'sanitize_options'],
        ]);
    }

    public static function sanitize_options($input): array {
        $input = is_array($input) ? $input : [];
        $issuer = isset($input['issuer']) ? untrailingslashit(esc_url_raw($input['issuer'])) : '';
        if ($issuer !== '' && strpos($issuer, 'https://') !== 0 && strpos($issuer, 'http://localhost') !== 0) {
            add_settings_error(self::OPTION, 'issuer', 'Auctor issuer must use HTTPS.');
            $issuer = '';
        }

        return [
            'issuer' => $issuer,
            'client_id' => isset($input['client_id'])
                ? sanitize_text_field($input['client_id'])
                : '',
            'allow_registration' => empty($input['allow_registration']) ? 0 : 1,
            'force_login' => empty($input['force_login']) ? 0 : 1,
            'button_label' => isset($input['button_label']) && trim($input['button_label']) !== ''
                ? sanitize_text_field($input['button_label'])
                : self::defaults()['button_label'],
        ];
    }

    public static function admin_menu(): void {
        add_options_page(
            'Auctor SSO',
            'Auctor SSO',
            'manage_options',
            'auctor-sso',
            [__CLASS__, 'settings_page']
        );
    }

    public static function settings_page(): void {
        if (!current_user_can('manage_options')) {
            return;
        }
        $options = self::options();
        ?>
        <div class="wrap">
            <h1>Auctor SSO</h1>
            <p>Authorization Code + PKCE integration. No client secret is stored in WordPress.</p>
            <?php settings_errors(self::OPTION); ?>
            <form method="post" action="options.php">
                <?php settings_fields('auctor_sso'); ?>
                <table class="form-table" role="presentation">
                    <tr>
                        <th><label for="auctor-issuer">Auctor URL</label></th>
                        <td>
                            <input id="auctor-issuer" class="regular-text" type="url"
                                   name="<?php echo esc_attr(self::OPTION); ?>[issuer]"
                                   value="<?php echo esc_attr($options['issuer']); ?>"
                                   placeholder="https://account.example.com" />
                        </td>
                    </tr>
                    <tr>
                        <th><label for="auctor-client-id">Client ID</label></th>
                        <td>
                            <input id="auctor-client-id" class="regular-text" type="text"
                                   name="<?php echo esc_attr(self::OPTION); ?>[client_id]"
                                   value="<?php echo esc_attr($options['client_id']); ?>" />
                        </td>
                    </tr>
                    <tr>
                        <th>Callback URL</th>
                        <td><code><?php echo esc_html(self::callback_url()); ?></code></td>
                    </tr>
                    <tr>
                        <th>New users</th>
                        <td>
                            <label>
                                <input type="checkbox"
                                       name="<?php echo esc_attr(self::OPTION); ?>[allow_registration]"
                                       value="1" <?php checked($options['allow_registration'], 1); ?> />
                                Create a WordPress subscriber when no linked/local account exists.
                            </label>
                        </td>
                    </tr>
                    <tr>
                        <th>WordPress login</th>
                        <td>
                            <label>
                                <input type="checkbox"
                                       name="<?php echo esc_attr(self::OPTION); ?>[force_login]"
                                       value="1" <?php checked($options['force_login'], 1); ?> />
                                Redirect the normal login form to Auctor.
                            </label>
                            <p class="description">
                                Add ?auctor_local=1 to wp-login.php to use the local WordPress
                                form for emergency maintenance.
                            </p>
                        </td>
                    </tr>
                    <tr>
                        <th><label for="auctor-button-label">Button label</label></th>
                        <td>
                            <input id="auctor-button-label" class="regular-text" type="text"
                                   name="<?php echo esc_attr(self::OPTION); ?>[button_label]"
                                   value="<?php echo esc_attr($options['button_label']); ?>" />
                        </td>
                    </tr>
                </table>
                <?php submit_button(); ?>
            </form>
        </div>
        <?php
    }

    public static function login_url(string $redirect = ''): string {
        $target = home_url('/?auth=auctor');
        if ($redirect !== '') {
            $target = add_query_arg('redirect_to', rawurlencode($redirect), $target);
        }
        return $target;
    }

    public static function render_login_button(): void {
        $options = self::options();
        if ($options['issuer'] === '' || $options['client_id'] === '') {
            return;
        }
        $redirect = isset($_REQUEST['redirect_to']) ? wp_unslash($_REQUEST['redirect_to']) : admin_url();
        echo '<p style="margin-top:16px">';
        echo '<a class="button button-primary button-large" style="width:100%;text-align:center" href="' .
            esc_url(self::login_url($redirect)) . '">' .
            esc_html($options['button_label']) . '</a>';
        echo '</p>';
    }

    public static function shortcode($atts): string {
        $options = self::options();
        $atts = shortcode_atts([
            'text' => $options['button_label'],
            'redirect_to' => home_url('/'),
            'class' => 'button button-primary auctor-sso-button',
        ], $atts, 'auctor_login_button');

        return sprintf(
            '<a class="%s" href="%s">%s</a>',
            esc_attr($atts['class']),
            esc_url(self::login_url($atts['redirect_to'])),
            esc_html($atts['text'])
        );
    }

    public static function maybe_force_login(): void {
        $options = self::options();
        if (empty($options['force_login']) || isset($_GET['auctor_local'])) {
            return;
        }
        $action = isset($_REQUEST['action']) ? sanitize_key($_REQUEST['action']) : 'login';
        if (!in_array($action, ['login', 'reauth'], true)) {
            return;
        }
        $redirect = isset($_REQUEST['redirect_to'])
            ? wp_unslash($_REQUEST['redirect_to'])
            : admin_url();
        wp_safe_redirect(self::login_url($redirect));
        exit;
    }

    public static function handle_callback(): void {
        if (!isset($_GET['auth']) || $_GET['auth'] !== 'auctor') {
            return;
        }

        $options = self::options();
        if ($options['issuer'] === '' || $options['client_id'] === '') {
            wp_die('Auctor SSO is not configured.');
        }

        if (isset($_GET['error'])) {
            wp_die('Auctor authorization failed: ' . esc_html(sanitize_text_field($_GET['error'])));
        }

        if (!isset($_GET['code'])) {
            self::begin_authorization($options);
        }

        self::complete_authorization($options);
    }

    private static function begin_authorization(array $options): void {
        $state = bin2hex(random_bytes(24));
        $verifier = self::base64url(random_bytes(64));
        $challenge = self::base64url(hash('sha256', $verifier, true));

        $redirect = isset($_GET['redirect_to'])
            ? rawurldecode(sanitize_text_field(wp_unslash($_GET['redirect_to'])))
            : home_url('/');

        set_transient(self::TRANSIENT_PREFIX . $state, [
            'verifier' => $verifier,
            'redirect' => wp_validate_redirect($redirect, home_url('/')),
        ], 10 * MINUTE_IN_SECONDS);

        $authorize = add_query_arg([
            'response_type' => 'code',
            'client_id' => $options['client_id'],
            'redirect_uri' => self::callback_url(),
            'state' => $state,
            'scope' => 'profile email groups roles',
            'code_challenge' => $challenge,
            'code_challenge_method' => 'S256',
        ], $options['issuer'] . '/oauth/authorize');

        wp_redirect(esc_url_raw($authorize));
        exit;
    }

    private static function complete_authorization(array $options): void {
        $state = isset($_GET['state']) ? sanitize_text_field(wp_unslash($_GET['state'])) : '';
        $code = sanitize_text_field(wp_unslash($_GET['code']));
        if ($state === '' || $code === '') {
            wp_die('Invalid Auctor callback.');
        }

        $flow = get_transient(self::TRANSIENT_PREFIX . $state);
        delete_transient(self::TRANSIENT_PREFIX . $state);
        if (!is_array($flow) || empty($flow['verifier'])) {
            wp_die('Auctor login expired or the state value is invalid.');
        }

        $token_response = wp_remote_post($options['issuer'] . '/oauth/token', [
            'timeout' => 15,
            'redirection' => 0,
            'sslverify' => true,
            'body' => [
                'grant_type' => 'authorization_code',
                'code' => $code,
                'client_id' => $options['client_id'],
                'redirect_uri' => self::callback_url(),
                'code_verifier' => $flow['verifier'],
            ],
        ]);

        if (is_wp_error($token_response)) {
            wp_die('Could not contact Auctor token endpoint.');
        }
        $token_body = json_decode(wp_remote_retrieve_body($token_response), true);
        if (wp_remote_retrieve_response_code($token_response) !== 200 || empty($token_body['access_token'])) {
            wp_die('Auctor token exchange failed.');
        }

        $info_response = wp_remote_get($options['issuer'] . '/oauth/userinfo', [
            'timeout' => 15,
            'redirection' => 0,
            'sslverify' => true,
            'headers' => [
                'Authorization' => 'Bearer ' . $token_body['access_token'],
                'Accept' => 'application/json',
            ],
        ]);
        if (is_wp_error($info_response)) {
            wp_die('Could not load the Auctor user profile.');
        }

        $profile = json_decode(wp_remote_retrieve_body($info_response), true);
        if (wp_remote_retrieve_response_code($info_response) !== 200 || empty($profile['sub'])) {
            wp_die('Auctor user profile is invalid.');
        }

        $user = self::resolve_user($profile, !empty($options['allow_registration']));
        if (is_wp_error($user)) {
            wp_die(esc_html($user->get_error_message()));
        }

        update_user_meta($user->ID, self::META_SUB, sanitize_text_field($profile['sub']));
        if (!empty($profile['name']) && $user->display_name !== $profile['name']) {
            wp_update_user([
                'ID' => $user->ID,
                'display_name' => sanitize_text_field($profile['name']),
            ]);
        }

        wp_clear_auth_cookie();
        wp_set_current_user($user->ID);
        wp_set_auth_cookie($user->ID, true, is_ssl());

        $redirect = isset($flow['redirect'])
            ? wp_validate_redirect($flow['redirect'], home_url('/'))
            : home_url('/');
        wp_safe_redirect($redirect);
        exit;
    }

    private static function resolve_user(array $profile, bool $allow_registration) {
        $linked = get_users([
            'meta_key' => self::META_SUB,
            'meta_value' => sanitize_text_field($profile['sub']),
            'number' => 1,
            'count_total' => false,
        ]);
        if (!empty($linked)) {
            return $linked[0];
        }

        $username = isset($profile['preferred_username'])
            ? sanitize_user($profile['preferred_username'], true)
            : '';
        $email = isset($profile['email']) ? sanitize_email($profile['email']) : '';

        $user = $username !== '' ? get_user_by('login', $username) : false;
        if (!$user && $email !== '') {
            $user = get_user_by('email', $email);
        }
        if ($user) {
            return $user;
        }

        if (!$allow_registration) {
            return new WP_Error('auctor_registration_disabled', 'No linked WordPress account exists.');
        }
        if ($username === '' || $email === '') {
            return new WP_Error('auctor_profile_incomplete', 'Auctor profile does not contain a usable username and email.');
        }

        $candidate = $username;
        $suffix = 2;
        while (username_exists($candidate)) {
            $candidate = $username . '-' . $suffix++;
        }

        $id = wp_insert_user([
            'user_login' => $candidate,
            'user_email' => $email,
            'display_name' => !empty($profile['name']) ? sanitize_text_field($profile['name']) : $candidate,
            'user_pass' => wp_generate_password(32, true, true),
            'role' => get_option('default_role', 'subscriber'),
        ]);
        if (is_wp_error($id)) {
            return $id;
        }
        return get_user_by('id', $id);
    }

    private static function base64url(string $raw): string {
        return rtrim(strtr(base64_encode($raw), '+/', '-_'), '=');
    }
}

Auctor_SSO::init();
