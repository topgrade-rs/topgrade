# Contributing to `topgrade`

Thank you for your interest in contributing to `topgrade`! We welcome and encourage contributions of all kinds, such as:

1. Issue reports or feature requests
2. Documentation improvements
3. Code (PR or PR Review)

Please follow the [Karma Runner guidelines](http://karma-runner.github.io/6.2/dev/git-commit-msg.html) for commit
messages.

## Adding a new step

In Topgrade's terminology, a package manager is called a "step". To add a new step to Topgrade:

1. Add a new variant to [`enum Step`](https://github.com/topgrade-rs/topgrade/blob/main/src/step.rs)

   ```rust
   pub enum Step {
       // Existing steps
       // ...

       // Your new step here!
       // Make sure it stays sorted alphabetically because that looks great :)
       Xxx,
   }
   ```

2. Implement the update function

   Find the appropriate location for this update function. It should be a file under
   [`src/steps`](https://github.com/topgrade-rs/topgrade/tree/main/src/steps); the file names are self-explanatory, for
   example, `step`s related to `zsh` are placed in
   [`steps/zsh.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/steps/zsh.rs).

   Then implement the update function, and put it in the file where it belongs.

   ```rust
   pub fn run_xxx(ctx: &ExecutionContext) -> Result<()> {
       // Check if this step is installed, if not, then this update will be skipped.
       let xxx = require("xxx")?;

       // Print the separator
       print_separator("xxx");

       // Invoke the new step to get things updated!
       ctx.execute(xxx)
          .arg(/* args required by this step */)
          .status_checked()
   }
   ```

   Such an update function is conventionally named `run_xxx()`, where `xxx` is the name of the new step. It should take
   an argument of type `&ExecutionContext`. This is adequate for most cases unless some extra work is needed (you can
   find examples where extra arguments are needed
   [in the main.rs file](https://github.com/topgrade-rs/topgrade/blob/7e48c5dedcfd5d0124bb9f39079a03e27ed23886/src/main.rs#L201-L219)).

   The update function usually does three things:
   1. Check if the step is installed.
   2. Output the separator.
   3. Invoke the step.

   This is sufficient for most tools, but you may need additional handling for more complicated steps.

3. Add a match arm to `Step::run()`

   ```rust
   Xxx => runner.execute(*self, "xxx", || ItsModule::run_xxx(ctx))?,
   ```

   We use [conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html) to separate steps.
   For example, for a Linux-only step:

   ```rust
   #[cfg(target_os = "linux")]
   {
       // Xxx is Linux-only
       runner.execute(Step::Xxx, "xxx", || ItsModule::run_xxx(&ctx))?;
   }
   ```

4. Finally, add the step to `default_steps()` in `step.rs`

   ```rust
   steps.push(Xxx)
   ```

   Try to keep the conditional compilation consistent with step 3 above.

   Congrats, you just added a new `step` :)

## Modifications to configuration entries

If your PR has the configuration options (in
[`src/config.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/config.rs)) modified:

1. Adding new options
2. Changing the existing options

Be sure to apply your changes to
[`config.example.toml`](https://github.com/topgrade-rs/topgrade/blob/main/config.example.toml), and include basic
documentation guiding users on how to use these options.

## Breaking changes

If your PR introduces a breaking change, document it in [`BREAKINGCHANGES_dev.md`][bc_dev]. It should be written in
Markdown and wrapped at 80 characters. For example:

```md
1. The configuration location has been updated to x.

2. The step x has been removed.

3. ...
```

[bc_dev]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES_dev.md

## Before you submit your PR

Make sure your patch passes the following tests on your host:

```shell
cargo build
cargo fmt
cargo clippy
cargo test
```

Don't worry about other platforms, we have most of them covered in our CI.

## Quick docs and lint checks

Before opening a PR, it helps to run the lightweight docs/lint checks locally:

- Format Markdown/JSON/TOML: `dprint fmt`
- Spell check: `typos`
- Link check: `lychee --config .lychee.toml --quiet .`

If you prefer a single command and you use cargo-make, you can run:

```sh
cargo make docs-check
```

Tip: there’s also a formatter task you can run before committing:

```sh
cargo make docs-fix
```

Notes

- These commands assume the tools are installed and on your PATH. They’re all Rust-based and installable via Cargo
  (for example: `cargo install dprint typos-cli lychee`).
- Our CI will also run these checks, so running them locally just saves you a round trip.

## I18n

If your PR introduces user-facing messages, we need to ensure they are translated. Please add the translations to
[`locales/app.yml`][app_yml]. For simple messages without arguments (e.g., "hello world"), we can simply translate them
accordingly (Tip: ChatGPT or similar LLMs are good at translation). If a message contains arguments, e.g., "hello
{NAME}", please follow this convention:

```yml
"hello {name}": # key
  en: "hello %{name}"  # translation
```

Arguments in the key should be in the format `{argument_name}`, and they will have the preceding `%` when used in
translations.

[app_yml]: https://github.com/topgrade-rs/topgrade/blob/main/locales/app.yml

## Some tips

1. Locale

   Some `step`s respect locale, which means their output can be in a language other than English, so we should not check
   against it.

   For example, one may want to check if a tool works by doing this:

   ```rust
   let output = Command::new("xxx").arg("--help").output().unwrap();
   let stdout = from_utf8(output.stdout).expect("Assume it is UTF-8 encoded");

   if stdout.contains("help") {
       // xxx works
   }
   ```

   If `xxx` respects locale, then the above code should work on an English system, but on a system that does not use
   English, e.g., it uses Chinese, that `"help"` may be translated to `"帮助"`, and the above code won't work.
