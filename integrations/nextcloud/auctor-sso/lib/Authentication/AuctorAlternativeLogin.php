<?php
declare(strict_types=1);

namespace OCA\AuctorSso\Authentication;

use OCA\AuctorSso\AppInfo\Application;
use OCP\Authentication\IAlternativeLogin;
use OCP\IConfig;
use OCP\IURLGenerator;

final class AuctorAlternativeLogin implements IAlternativeLogin {
    public function __construct(
        private IURLGenerator $urlGenerator,
        private IConfig $config,
    ) {
    }

    public function getLabel(): string {
        $label = trim($this->config->getAppValue(Application::APP_ID, 'login_label', ''));
        return $label !== '' ? $label : 'Sign in with Auctor';
    }

    public function getLink(): string {
        return $this->urlGenerator->linkToRoute('auctor_sso.auth.login');
    }

    public function getClass(): string {
        return 'auctor-sso-login';
    }

    public function load(): void {
    }
}
