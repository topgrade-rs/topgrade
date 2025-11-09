## Contributing to `topgrade`

Thank you for your interest in contributing to `topgrade`!
We welcome and encourage contributions of all kinds, such as:

1. Issue reports or feature requests
2. Documentation improvements
3. Code (PR or PR Review)

### LLM/AI guidelines

You may use LLMs (AI tools) for:

* Inspiration, problem solving, learning Rust, translation (to English or to Rust), etc.
* Generating small and self-contained snippets of code (e.g., shell scripts or utility functions)

Do **not** use LLMs to:

* Generate ("vibe code") entire pull requests
* Write or generate issue or pull request descriptions

### General guidelines

**Please use [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) for your PR title**.

We use [pre-commit](https://github.com/pre-commit/pre-commit). It runs in CI, but you can optionally install the hook
locally with `pre-commit install`. If you don't want to use pre-commit, make sure the following pass before submitting
your PR:

```shell
$ cargo fmt
$ cargo clippy
$ cargo test
```

### Adding a new step

In `topgrade`'s terms, a package manager (or something else that can be upgraded) is called a step.
To add a new step to `topgrade`:

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
   the file names are self-explanatory, for example, steps related to `zsh` are
   placed in [`steps/zsh.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/steps/zsh.rs), and steps that run on
   Linux only are placed in [`steps/linux.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/steps/linux.rs).

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

   Such an update function would be conventionally named `run_xxx()`, where `xxx`
   is the name of the new step, and it should take an argument of type
   `&ExecutionContext`.

   The update function should usually do 3 things:
    1. Check if the step is installed
    2. Output the separator
    3. Execute commands

   This is sufficient for most tools, but you may need some extra stuff
   for complicated steps.

3. Add a match arm to `Step::run()`

   ```rust
   Xxx => runner.execute(*self, "xxx", || ItsModule::run_xxx(ctx))?
   ```

   We use [conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
   to separate the steps. For example, for steps that are Linux-only, it goes
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
   Keep the conditional compilation the same as in the above step 3.

   Congrats, you just added a new step :)

### Modification to the configuration entries

If your PR has the configuration options
(in [`src/config.rs`](https://github.com/topgrade-rs/topgrade/blob/main/src/config.rs))
modified:

1. Adding new options
2. Changing the existing options

Be sure to apply your changes to
[`config.example.toml`](https://github.com/topgrade-rs/topgrade/blob/main/config.example.toml),
and have some basic documentations guiding user how to use these options.

### Breaking changes

If your PR introduces a breaking change, document it in [`BREAKINGCHANGES_dev.md`][bc_dev].
It should be written in Markdown and wrapped at 80, for example:

```md
1. The configuration location has been updated to x.

2. The step x has been removed.

3. ...
```

[bc_dev]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES_dev.md

### I18n

If your PR introduces user-facing messages, we need to ensure they are translated.
Please add the translations to [`locales/app.yml`][app_yml]. For simple messages
without arguments (e.g., "hello world"), we can simply translate them according
(Tip: LLMs are good at translation). If a message contains
arguments, e.g., "hello <NAME>", please follow this convention:

```yml
"hello {name}": # key
  en: "hello %{name}"  # translation
```

Arguments in the key should be in format `{argument_name}`, and they will have
a preceding `%` when used in translations.

[app_yml]: https://github.com/topgrade-rs/topgrade/blob/main/locales/app.yml

### Locales

Some steps respect locale, which means their output can be in language other
than English. In those cases, we cannot rely on the output of a command.

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
