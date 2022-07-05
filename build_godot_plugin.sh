#!/bin/bash

cargo build --release -p blackjack_godot
mkdir -p target/
cp -r godot_plugin target/
cp -r node_libraries target/godot_plugin
cp target/release/libblackjack_godot.so target/godot_plugin/addons/blackjack_engine_godot/

cd target/godot_plugin
rm target/blackjack_engine_godot.zip
zip -r ../blackjack_engine_godot.zip *
