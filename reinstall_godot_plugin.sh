#!/bin/bash

GODOT_PROJECT=$HOME/MY_GODOT_PROJECT

./build_godot_plugin.sh && cp target/blackjack_engine_godot.zip $GODOT_PROJECT && pushd $GODOT_PROJECT && rm -r addons; rm -r node_libraries; unzip blackjack_engine_godot.zip; rm blackjack_engine_godot.zip && popd
