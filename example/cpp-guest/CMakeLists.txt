cmake_minimum_required(VERSION 3.10)
project(wasm96-cpp-example)

# Set the C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Add the executable
add_executable(wasm96-example main.cpp)

# Set output to .wasm and export functions
set_target_properties(wasm96-example PROPERTIES
    SUFFIX ".wasm"
    LINK_FLAGS "--export=setup --export=update --export=draw"
)