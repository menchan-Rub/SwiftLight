#!/bin/bash
cd /home/menchan/Programming/SwiftLight
echo "Cargo.lock からインクウェルのバージョンを確認"
grep -A 3 "name = \"inkwell\"" Cargo.lock
grep -n "build_struct_gep" crates/swiftlight-compiler/src/middleend/ir/mod.rs.bak | head -5
echo -e "\nbuild_load関数:"
grep -n "build_load" crates/swiftlight-compiler/src/middleend/ir/mod.rs.bak | head -5
