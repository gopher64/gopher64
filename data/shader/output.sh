#!/bin/bash

glslc -O crt-aperture.frag -o frag.spv --target-env=vulkan1.1 -mfmt=c
