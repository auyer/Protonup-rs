complete -c protonup-rs -l tool -d 'Compatibility tool to install (e.g., GEProton, Luxtorpeda)' -r
complete -c protonup-rs -l version -d 'Version to install (use "latest" for the latest version)' -r
complete -c protonup-rs -l for -d 'Target for installation. Use "steam", "lutris", or a custom path. If omitted, auto-detects Steam or Lutris' -r
complete -c protonup-rs -l arch -d 'Architecture to assume for release assets without an architecture suffix. Accepts amd64/x86_64/x86 or arm64/arm/aarch64. Defaults to amd64' -r
complete -c protonup-rs -s q -l quick-download -d 'Skip Menu, auto detect apps and download using default parameters'
complete -c protonup-rs -s f -l force -d 'Force install for existing apps during quick downloads'
complete -c protonup-rs -s w -l whats-new -d 'Show release notes for latest versions of default tools'
complete -c protonup-rs -s h -l help -d 'Print help'
