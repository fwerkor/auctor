<?php
declare(strict_types=1);

namespace OCA\AuctorSso\Listener;

use OCA\AuctorSso\AppInfo\Application;
use OC\User\LoginException;
use OCP\EventDispatcher\Event;
use OCP\EventDispatcher\IEventListener;
use OCP\IConfig;
use OCP\IRequest;
use OCP\User\Events\BeforeUserLoggedInEvent;

/**
 * @template-implements IEventListener<BeforeUserLoggedInEvent>
 */
final class LocalPasswordLoginBlocker implements IEventListener {
    public function __construct(
        private IRequest $request,
        private IConfig $config,
    ) {
    }

    public function handle(Event $event): void {
        if (!$event instanceof BeforeUserLoggedInEvent) {
            return;
        }

        if (!self::appBool($this->config, 'sso_only', false)) {
            return;
        }

        if (strtoupper($this->request->getMethod()) !== 'POST') {
            return;
        }

        $path = rtrim((string)$this->request->getPathInfo(), '/');
        if ($path !== '/login') {
            return;
        }

        throw new LoginException('Password login is disabled. Sign in with Auctor.');
    }

    private static function appBool(IConfig $config, string $key, bool $default): bool {
        $value = strtolower(trim($config->getAppValue(
            Application::APP_ID,
            $key,
            $default ? 'true' : 'false',
        )));
        return in_array($value, ['1', 'true', 'yes', 'on'], true);
    }
}
