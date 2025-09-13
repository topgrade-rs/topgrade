## Contributing to `topgrade`

Thank you for your interest in contributing to `topgrade`!
We welcome and encourage contributions of all kinds, such as:

1. Issue reports or feature requests
2. Documentation improvements
3. Code (PR or PR Review)

Please follow the [Karma Runner guidelines](http://karma-runner.github.io/6.2/dev/git-commit-msg.html)
for commit messages.

## Adding a new `step`

In `topgrade`'s term, package manager is called `step`.
To add a new `step` to `topgrade`:

1. Add a new variant to
   [`enum Step`](https://github.com/topgrade-rs/topgrade/blob/main/src/step.rs)

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

   You need to find the appropriate location where this update function goes, it should be
   a file under [`src/steps`](https://github.com/topgrade-rs/topgrade/tree/main/src/steps),
   the file names are self-explanatory, for example, `step`s related to `zsh` are
   placed in [`steps/zsh.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/steps/zsh.rs).

   Then you implement the update function, and put it in the file where it belongs.

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

   Such a update function would be conventionally named `run_xxx()`, where `xxx`
   is the name of the new step, and it should take a argument of type
   `&ExecutionContext`, this is adequate for most cases unless some extra stuff is
   needed (You can find some examples where extra arguments are needed
   [here](https://github.com/topgrade-rs/topgrade/blob/7e48c5dedcfd5d0124bb9f39079a03e27ed23886/src/main.rs#L201-L219)).

   Update function would usually do 3 things:
    1. Check if the step is installed
    2. Output the Separator
    3. Invoke the step

   Still, this is sufficient for most tools, but you may need some extra stuff
   with complicated `step`.

3. Add a match arm to `Step::run()`

   ```rust
   Xxx => runner.execute(*self, "xxx", || ItsModule::run_xxx(ctx))?
   ```

   We use [conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
   to separate the steps, for example, for steps that are Linux-only, it goes
   like this:

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
   Try to keep the conditional compilation the same as in the above step 3.

   Congrats, you just added a new `step` :)

## Modification to the configuration entries

If your PR has the configuration options
(in [`src/config.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/config.rs))
modified:

1. Adding new options
2. Changing the existing options

Be sure to apply your changes to
[`config.example.toml`](https://github.com/topgrade-rs/topgrade/blob/main/config.example.toml),
and have some basic documentations guiding user how to use these options.

## Breaking changes

If your PR introduces a breaking change, document it in [`BREAKINGCHANGES_dev.md`][bc_dev],
it should be written in Markdown and wrapped at 80, for example:

```md
1. The configuration location has been updated to x.

2. The step x has been removed.

3. ...
```

[bc_dev]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES_dev.md

## Before you submit your PR

Make sure your patch passes the following tests on your host:

```shell
$ cargo build
$ cargo fmt
$ cargo clippy
$ cargo test
```

Don't worry about other platforms, we have most of them covered in our CI.

### Tooling and pre-commit (WSL recommended)

We use pre-commit to run a small set of checks/fixers locally:

- Shell scripts: shellcheck
- Whitespace/EOF fixers
- Markdown/JSON formatting: dprint
- Optional/Manual: gitleaks (secret scan)

Tips:

- Linux/WSL is the most reliable environment to run pre-commit.
- Use pre-commit 4.x or newer.

Quick start on Debian/WSL (one-time):

```bash
sudo apt-get update -y
sudo apt-get install -y pipx unzip curl git
pipx install pre-commit==4.3.0
curl -fsSL https://dprint.dev/install.sh | sh -s 0.50.1
```

Run the hooks in the repo:

```bash
pre-commit clean
pre-commit run --all-files
```

Formatting only (dprint):

```bash
~/.dprint/bin/dprint fmt
```

Secret scan (manual stage only):

```bash
gitleaks protect --staged --redact
# or via pre-commit’s manual stage
pre-commit run --hook-stage manual gitleaks
```

Notes:

- On Windows, upstream hooks may fail due to MSYS “fork” issues. Prefer WSL.
- Our CI runs pre-commit via pre-commit-ci. dprint is intentionally skipped in pre-commit-ci; if CI enforcement is desired later, we can add the dprint/check GitHub Action (Linux-only).
- .gitignore intentionally ignores only pre-commit cache artifacts (kept narrow) to avoid masking other local tool caches.

## I18n

If your PR introduces user-facing messages, we need to ensure they are translated.
Please add the translations to [`locales/app.yml`][app_yml]. For simple messages
without arguments (e.g., "hello world"), we can simply translate them according
(Tip: ChatGPT or similar LLMs is good at translation). If a message contains
arguments, e.g., "hello `NAME`", please follow this convention:

```yml
"hello {name}": # key
  en: "hello %{name}"  # translation
```

Arguments in the key should be in format `{argument_name}`, and they will have
a preceding `%` when used in translations.

[app_yml]: https://github.com/topgrade-rs/topgrade/blob/main/locales/app.yml

## Some tips

1. Locale

   Some `step` respects locale, which means their output can be in language other
   than English, we should not do check on it.

   For example, one may want to check if a tool works by doing this:

   ```rust
   let output = Command::new("xxx").arg("--help").output().unwrap();
   let stdout = from_utf8(output.stdout).expect("Assume it is UTF-8 encoded");

   if stdout.contains("help") {
       // xxx works
   }
   ```

   If `xxx` respects locale, then the above code should work on English system,
   on a system that does not use English, e.g., it uses Chinese, that `"help"` may be
   translated to `"帮助"`, and the above code won't work.
