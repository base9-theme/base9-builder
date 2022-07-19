# Official Base9 Builder

## CLI

Help page:
```
$ base9-builder help
base9 builder CLI

USAGE:
    base9-builder <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    help       Print this message or the help of the given subcommand(s)
    preview    prints a table of all generated colors to preview
    render     renders theme template
```

Example commands:
```bash
PALETTE="282936-E9E9F4-FF5555-FFB86C-F1FA8C-50FA7B-8BE9FD-BD93F9-FF79C6"

# preivew default palette in stdout
base9-builder preview -

# render mustache in stdout
base9-builder render $PALETTE template.mustache

# render mustache to file
base9-builder render $PALETTE template.mustache out.txt
```

## NPM/WASM/Typescript

```ts
import init, * as base9 from 'base9-builder';
// Required to setup wasm
await init();

const palette = "282936-E9E9F4-FF5555-FFB86C-F1FA8C-50FA7B-8BE9FD-BD93F9-FF79C6"

const colors = base9.get_colors(palette);
console.log(colors.red.p100); // #ff5555

const data = base9.get_data(palette);
console.log(data.foreground.p100.hex) // e9e9f4

const template = "foreground: {{foreground.p100.hex}}";
const rendered = base9.render_str(palette, template);
console.log(rendered); // foreground: e9e9f4

```

## Node.js

Not supported

## Rust crate

Comming soon.

