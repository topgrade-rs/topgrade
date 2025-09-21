TODO: For major versions, we want to bypass release-plz and do this

   1. bumps the version number.

      > If there are breaking changes, the major version number should be increased.

   2. If the major versioin number gets bumped, update [SECURITY.md][SECURITY_file_link].

      [SECURITY_file_link]: https://github.com/topgrade-rs/topgrade/blob/main/SECURITY.md

   3. Overwrite [`BREAKINGCHANGES`][breaking_changes] with
      [`BREAKINGCHANGES_dev`][breaking_changes_dev], and create a new dev file:

      ```sh'
      $ cd topgrade
      $ mv BREAKINGCHANGES_dev.md BREAKINGCHANGES.md
      $ touch BREAKINGCHANGES_dev.md
      ```

      [breaking_changes_dev]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES_dev.md
      [breaking_changes]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES.md
