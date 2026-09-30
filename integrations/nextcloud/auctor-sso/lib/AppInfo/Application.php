<?php
declare(strict_types=1);

namespace OCA\AuctorSso\AppInfo;

use OCA\AuctorSso\Authentication\AuctorAlternativeLogin;
use OCA\AuctorSso\Listener\LocalPasswordLoginBlocker;
use OCP\AppFramework\App;
use OCP\AppFramework\Bootstrap\IBootContext;
use OCP\AppFramework\Bootstrap\IBootstrap;
use OCP\AppFramework\Bootstrap\IRegistrationContext;
use OCP\User\Events\BeforeUserLoggedInEvent;

final class Application extends App implements IBootstrap {
    public const APP_ID = 'auctor_sso';

    public function __construct() {
        parent::__construct(self::APP_ID);
    }

    public function register(IRegistrationContext $context): void {
        $context->registerAlternativeLogin(AuctorAlternativeLogin::class);
        $context->registerEventListener(BeforeUserLoggedInEvent::class, LocalPasswordLoginBlocker::class);
    }

    public function boot(IBootContext $context): void {
    }
}
