project(
  'caps-lock-daemon',
  'c',
  version: '1.0.0',
  default_options: [
    'c_std=c23',
    'warning_level=3',
  ],
)

executable(
  'caps-lock-daemon',
  'main.c',
  dependencies: [
    dependency('xkbcommon'),
  ],
  install: true,
  install_dir: join_paths(
    get_option('prefix'),
    get_option('bindir'),
  ),
)

subdir('systemd')
