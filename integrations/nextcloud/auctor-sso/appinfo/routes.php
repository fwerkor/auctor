<?php
declare(strict_types=1);

return [
    'routes' => [
        ['name' => 'auth#login', 'url' => '/login', 'verb' => 'GET'],
        ['name' => 'auth#callback', 'url' => '/callback', 'verb' => 'GET'],
    ],
];
