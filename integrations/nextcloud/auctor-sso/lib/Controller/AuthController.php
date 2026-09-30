<?php
declare(strict_types=1);

namespace OCA\AuctorSso\Controller;

use OCA\AuctorSso\AppInfo\Application;
use OCP\AppFramework\Controller;
use OCP\AppFramework\Http\Attribute\NoCSRFRequired;
use OCP\AppFramework\Http\Attribute\PublicPage;
use OCP\AppFramework\Http\Attribute\UseSession;
use OCP\AppFramework\Http\DataResponse;
use OCP\AppFramework\Http\RedirectResponse;
use OCP\AppFramework\Http\Response;
use OCP\Http\Client\IClientService;
use OCP\IConfig;
use OCP\IRequest;
use OCP\ISession;
use OCP\IURLGenerator;
use OCP\IUser;
use OCP\IUserManager;
use OCP\IUserSession;

final class AuthController extends Controller {
    private const SESSION_KEY = 'auctor_sso_flow';

    public function __construct(
        string $appName,
        IRequest $request,
        private IURLGenerator $urlGenerator,
        private ISession $session,
        private IClientService $clientService,
        private IUserManager $userManager,
        private IUserSession $userSession,
        private IConfig $config,
    ) {
        parent::__construct($appName, $request);
    }

    #[NoCSRFRequired]
    #[PublicPage]
    #[UseSession]
    public function login(): Response {
        if ($this->userSession->isLoggedIn()) {
            return new RedirectResponse($this->urlGenerator->linkToDefaultPageUrl());
        }

        $issuer = rtrim($this->config->getAppValue(Application::APP_ID, 'issuer', ''), '/');
        $clientId = trim($this->config->getAppValue(Application::APP_ID, 'client_id', ''));
        if ($issuer === '' || $clientId === '') {
            return new DataResponse(['error' => 'Auctor SSO is not configured'], 500);
        }

        $state = bin2hex(random_bytes(24));
        $verifier = self::base64Url(random_bytes(64));
        $challenge = self::base64Url(hash('sha256', $verifier, true));
        $callback = $this->urlGenerator->linkToRouteAbsolute('auctor_sso.auth.callback');

        $this->session->set(self::SESSION_KEY, [
            'state' => $state,
            'verifier' => $verifier,
            'created' => time(),
        ]);

        $scope = trim($this->config->getAppValue(
            Application::APP_ID,
            'scope',
            'profile email groups roles',
        ));
        if ($scope === '') {
            $scope = 'profile email groups roles';
        }

        $query = http_build_query([
            'response_type' => 'code',
            'client_id' => $clientId,
            'redirect_uri' => $callback,
            'state' => $state,
            'scope' => $scope,
            'code_challenge' => $challenge,
            'code_challenge_method' => 'S256',
        ], '', '&', PHP_QUERY_RFC3986);

        return new RedirectResponse($issuer . '/oauth/authorize?' . $query);
    }

    #[NoCSRFRequired]
    #[PublicPage]
    #[UseSession]
    public function callback(): Response {
        $error = (string)$this->request->getParam('error', '');
        if ($error !== '') {
            return new DataResponse(['error' => 'Auctor authorization failed', 'detail' => $error], 401);
        }

        $state = (string)$this->request->getParam('state', '');
        $code = (string)$this->request->getParam('code', '');
        $flow = $this->session->get(self::SESSION_KEY);
        $this->session->remove(self::SESSION_KEY);

        if (!is_array($flow)
            || $state === ''
            || $code === ''
            || !hash_equals((string)($flow['state'] ?? ''), $state)
            || empty($flow['verifier'])
            || (int)($flow['created'] ?? 0) < time() - 600) {
            return new DataResponse(['error' => 'Invalid or expired Auctor login state'], 400);
        }

        $issuer = rtrim($this->config->getAppValue(Application::APP_ID, 'issuer', ''), '/');
        $clientId = trim($this->config->getAppValue(Application::APP_ID, 'client_id', ''));
        if ($issuer === '' || $clientId === '') {
            return new DataResponse(['error' => 'Auctor SSO is not configured'], 500);
        }

        $callback = $this->urlGenerator->linkToRouteAbsolute('auctor_sso.auth.callback');
        $client = $this->clientService->newClient();

        try {
            $tokenResponse = $client->post($issuer . '/oauth/token', [
                'timeout' => 15,
                'allow_redirects' => false,
                'body' => [
                    'grant_type' => 'authorization_code',
                    'code' => $code,
                    'client_id' => $clientId,
                    'redirect_uri' => $callback,
                    'code_verifier' => (string)$flow['verifier'],
                ],
            ]);
        } catch (\Throwable $e) {
            return new DataResponse(['error' => 'Could not contact Auctor token endpoint'], 502);
        }

        $tokenBody = json_decode((string)$tokenResponse->getBody(), true);
        if ($tokenResponse->getStatusCode() !== 200 || !is_array($tokenBody) || empty($tokenBody['access_token'])) {
            return new DataResponse(['error' => 'Auctor token exchange failed'], 502);
        }

        try {
            $profileResponse = $client->get($issuer . '/oauth/userinfo', [
                'timeout' => 15,
                'allow_redirects' => false,
                'headers' => [
                    'Authorization' => 'Bearer ' . $tokenBody['access_token'],
                    'Accept' => 'application/json',
                ],
            ]);
        } catch (\Throwable $e) {
            return new DataResponse(['error' => 'Could not load Auctor user profile'], 502);
        }

        $profile = json_decode((string)$profileResponse->getBody(), true);
        if ($profileResponse->getStatusCode() !== 200 || !is_array($profile) || empty($profile['sub'])) {
            return new DataResponse(['error' => 'Auctor user profile is invalid'], 502);
        }

        $user = $this->resolveUser($profile);
        if ($user === null || !$user->isEnabled()) {
            return new DataResponse(['error' => 'No usable Nextcloud account for this Auctor identity'], 403);
        }

        $uid = $user->getUID();
        $this->config->setUserValue($uid, Application::APP_ID, 'auctor_sub', (string)$profile['sub']);

        if (!empty($profile['name'])) {
            $user->setDisplayName((string)$profile['name']);
        }
        if (!empty($profile['email'])) {
            $user->setEMailAddress((string)$profile['email']);
        }

        $this->userSession->setUser($user);
        return new RedirectResponse($this->urlGenerator->linkToDefaultPageUrl());
    }

    private function resolveUser(array $profile): ?IUser {
        $sub = (string)$profile['sub'];

        foreach ($this->userManager->search('', 1000, 0) as $candidate) {
            if ($this->config->getUserValue($candidate->getUID(), Application::APP_ID, 'auctor_sub', '') === $sub) {
                return $candidate;
            }
        }

        $username = trim((string)($profile['preferred_username'] ?? ''));
        if ($username !== '') {
            $existing = $this->userManager->get($username);
            if ($existing !== null) {
                return $existing;
            }
        }

        $email = trim((string)($profile['email'] ?? ''));
        if ($email !== '') {
            $matches = $this->userManager->getByEmail($email);
            if (!empty($matches)) {
                return $matches[0];
            }
        }

        if (!self::appBool($this->config, 'auto_create_users', true)) {
            return null;
        }

        if ($username === '' || !preg_match('/^[A-Za-z0-9_.@-]{1,64}$/', $username)) {
            return null;
        }

        return $this->userManager->createUser($username, bin2hex(random_bytes(32)));
    }

    private static function appBool(IConfig $config, string $key, bool $default): bool {
        $value = strtolower(trim($config->getAppValue(
            Application::APP_ID,
            $key,
            $default ? 'true' : 'false',
        )));
        return in_array($value, ['1', 'true', 'yes', 'on'], true);
    }

    private static function base64Url(string $value): string {
        return rtrim(strtr(base64_encode($value), '+/', '-_'), '=');
    }
}
