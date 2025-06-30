1. The `jet_brains_toolbox` step was renamed to `jetbrains_toolbox`. If you're
   using the old name in your configuration file in the `disable` or `only`
   fields, simply change it to `jetbrains_toolbox`.

2. The "dry run" command line argument has been replaced with the run mode
   argument:
    * `-n` => `-m dry`
    * `--dry-run` => `--run-type dry`