# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************

load("@cargo-workspace//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "abi-types",
    srcs = glob(["src/**/*.rs"]),
    deps = all_crate_deps(
        normal = True,
    ) + [
        "//src/codegen:abi-types-codegen",
        "//src/parser:abi-types-parser",
    ],
)
