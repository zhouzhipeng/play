## install target
```bash
rustup target add wasm32-unknown-unknown
```

## install  wasm-pack
```bash
brew install  wasm-pack
```

## pack
```bash
 wasm-pack build --target web
 ```

## copy to static
```bash
cp -r pkg ../server/static/wasm/pkg
```