#!/bin/bash

SHADER_DIR="assets/shaders"


echo "Compiling vertex shaders"

for vert_shader in "$SHADER_DIR"/*.vert.glsl; do
  if [ -f "$vert_shader" ]; then
    filename=$(basename -- "$vert_shader")
    filename_no_ext="${filename%.*}"
    if [ "$1" != "debug" ]; then
      output_dir="bin/assets/shaders"
      output_path="$output_dir/$filename_no_ext.spv"
      echo $output_path
      if [ -f "$output_path" ]; then
        echo "file exists: $output_path"
      else
        echo "creating file: $output_path"
        touch "$output_path"  
      fi
      echo "  Compiling $vert_shader to $output_path"
      glslc -fshader-stage=vertex "$vert_shader" -o "$output_path"
      if [ $? -ne 0 ]; then
        echo "Error compiling $vert_shader"
        exit 1
      fi
    else
      output_dir="bin/assets/shaders"
      output_path="$output_dir/$filename_no_ext.spv"
      echo $output_path
      if [ -f "$output_path" ]; then
        echo "file exists: $output_path"
      else
        echo "creating file: $output_path"
         touch "$output_path"  
      fi
      echo "  Compiling $vert_shader to $output_path"
      glslc -fshader-stage=vertex "$vert_shader" -o "$output_path"
      if [ $? -ne 0 ]; then
        echo "Error compiling $vert_shader"
        exit 1
      fi
    fi
  fi
done

echo "Compiling fragment shaders..."
for frag_shader in "$SHADER_DIR"/*.frag.glsl; do
  if [ -f "$frag_shader" ]; then
    filename=$(basename -- "$frag_shader")
    filename_no_ext="${filename%.*}"
    if [ "$1" != "debug" ]; then
      output_dir="bin/assets/shaders"
      output_path="$output_dir/$filename_no_ext.spv"
      if [ -f "$output_path" ]; then
        echo "file exists: $output_path"
      else
        echo "creating file: $output_path"
         touch "$output_path"  
      fi
      echo "  Compiling $frag_shader to $output_path"
      glslc -fshader-stage=fragment "$frag_shader" -o "$output_path"
      if [ $? -ne 0 ]; then
        echo "Error compiling $frag_shader"
        exit 1
      fi
    else
      output_dir="bin/assets/shaders"
      output_path="$output_dir/$filename_no_ext.spv"
      if [ -f "$output_path" ]; then
        echo "file exists: $output_path"
      else
        echo "creating file: $output_path"
         touch "$output_path"  
      fi
      echo "  Compiling $frag_shader to $output_path"
      glslc -fshader-stage=fragment "$frag_shader" -o "$output_path"
      if [ $? -ne 0 ]; then
        echo "Error compiling $frag_shader"
        exit 1
      fi
    fi
  fi
done

echo "Shader compilation complete."
