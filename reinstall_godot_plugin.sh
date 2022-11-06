#!/bin/bash

if [ -z "$1" ]
then
    echo "Expected an argument with the path to the project"
    exit 1
fi

GODOT_PROJECT="$1"

./build_godot_plugin.sh && \
    cp target/blackjack_engine_godot.zip $GODOT_PROJECT && \
    pushd $GODOT_PROJECT && \
    rm -r addons; \
    rm -r blackjack_lua; \
    unzip blackjack_engine_godot.zip; \
    rm blackjack_engine_godot.zip && \
    popd
