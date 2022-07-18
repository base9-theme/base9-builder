#!/bin/bash
cd "$(dirname "$0")"
DTS_FILE="../pkg/base9_builder.d.ts"

function fix_type() {
    echo "Changing $1 to return $2"
    sed -i "s/\($1(.*\)any;$/\1$2;/g" $DTS_FILE
}

echo "Monkey patching $DTS_FILE with custom return types"

fix_type "palette_to_data" "Data"
fix_type "palette_to_color_hash" "NestedObj<Formatted>"
