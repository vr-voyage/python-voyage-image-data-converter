BC7 and DXT5 conversion library for Python
------------------------------------------

Written in Rust, converted using PyO3, in order to use
it in a texture reconverter addon for Blender.

Only Tested on Windows at the moment.

# Compiling

To compile this projet, you'll need to use `maturin` like this :

```bash
maturin build --release
```

To target a specific version of python, you'll need to do :

```bash
maturin build --release --interpreter ${version}
```
> With `${version}` being something like `3.8` for Python 3.8
