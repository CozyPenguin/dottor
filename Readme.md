# Dottor

Dottor is a dotfiles manager written in Rust.
I created it because I needed a cross-platform (currently Windows and Linux, but support for other platforms is trivial to add) solution
for managing the configurations of the various programs I use.

## Using Dottor

To create a repository which is managed by dottor, you just need to run `dottor init` in the directory where you want to store your dotfiles.
This will create `dottor.toml` and optionally initialize a git repository if you have git installed.

`dottor.toml` is the file where all the options which apply to the whole repository are located.
Currently the only functional one is exclude, which takes a list of folders in which dottor doesn't look for configurations.

## Contributing

Dottor is still in early development, so feedback and contributions are very appreciated.

If you have found a bug, please open an [issue](https://github.com/cschierig/dottor/issues/new).

If you want to add a feature, you can open a pull request.
If you want to add something which requires changing existing systems or you are unsure if your idea fits the scope of the project,
you can contact me by any of the methods mentioned in my [Profile Readme](https://github.com/cschierig).
