#!/bin/bash

wasm-pack build --target web
tsc -b

rm -rf dist
mkdir -p dist/pkg
cp index.html index.js dist
cp -r icons dist
cp pkg/sdv_volcano.js pkg/sdv_volcano_bg.wasm dist/pkg
