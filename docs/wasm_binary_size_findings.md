# This documents the finding while trying to reduce wasm binary size:

## Guides followed:
https://github.com/johnthagen/min-sized-rust


## History

### [commit on 2022-07-23](https://github.com/base9-theme/base9-builder/tree/63190a91e77c9f1290f1edb20c0fe9143bb7593e)

Findings:

- `regex` is very big, `const_regex` is much smaller.
- `json` is much smaller than `yaml`.
- `lto`, `opt-level=z`, `codegen-units=1`, `wasm-opt` are all somewhat useful.
- `opt-level=s`, `strip`, `panic=abort` are not as useful.
- `mustache`, `JsValue::from_serde` are big, but necessary.

Data:

```
+ serde_yaml + regex: 1041kb
+ serde_yaml + regex + lto: 1041kb
+ serde_yaml + regex + lto + opt=s: 950kb
+ serde_yaml + regex + lto + opt=z: 877kb
+ serde_yaml + const_regex + lto + opt=z: 335kb
+ serde_yaml + const_regex + lto + opt=s: 371kb
+ serde_yaml + const_regex + lto + opt=z + strip: 337kb
+ serde_yaml + const_regex + lto + opt=z + codegen=1: 328kb
+ serde_yaml + const_regex + lto + opt=z + codegen=1 + panic=abort: 328kb
+ serde_json + const_regex + lto + opt=z + codegen=1: 225kb
```

Then with
```
serde_json + const_regex + lto + opt=z + codegen=1: 225kb
```
as default, I did more experiments:

```
- mustache: 185kb
- JsValue.from_serde: 209kb
+ wasm-opt-Oz: 218kb
```