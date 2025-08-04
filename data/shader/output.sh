#!/bin/bash

glslc -O crt-aperture.frag -o crt_frag.spv --target-env=vulkan1.1 -mfmt=c
glslc -O main.frag -o main_frag.spv --target-env=vulkan1.1 -mfmt=c
glslc -O main.vert -o main_vert.spv --target-env=vulkan1.1 -mfmt=c
glslc -O text.frag -o text_frag.spv --target-env=vulkan1.1 -mfmt=c
glslc -O text.vert -o text_vert.spv --target-env=vulkan1.1 -mfmt=c
