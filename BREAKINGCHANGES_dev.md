1. The `jet_brains_toolbox` step was renamed to `jetbrains_toolbox`. If you're
   using the old name in your configuration file in the `disable` or `only`
   fields, simply change it to `jetbrains_toolbox`.
2. The `nix_helper` step was deprecated. Its `home_manager`-replacing
   functionality has been merged into that step. When on NixOS, the `system`
   step will decide whether to use `nh` according to the new config option
   `misc.nix_handler`.
3. Since the `nix_helper` step is gone and `nix-darwin` support is otherwise
   untested, `nh darwin` support has been removed for the time being as well.
