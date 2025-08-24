cmake_minimum_required(VERSION 3.14)
project(PlayPlugins VERSION 1.0.0 LANGUAGES C CXX)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Set C standard
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Find required packages
find_package(CURL REQUIRED)
find_package(PkgConfig REQUIRED)

# Find json-c for C plugin
pkg_check_modules(JSONC REQUIRED json-c)

# Find jsoncpp for C++ plugin
find_package(jsoncpp REQUIRED)

# Create C plugin library
add_library(c_plugin SHARED example_plugin.c)
target_include_directories(c_plugin PRIVATE 
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${JSONC_INCLUDE_DIRS}
)
target_link_libraries(c_plugin 
    ${CURL_LIBRARIES}
    ${JSONC_LIBRARIES}
)
target_compile_options(c_plugin PRIVATE ${JSONC_CFLAGS_OTHER})

# Create C++ plugin library
add_library(cpp_plugin SHARED example_plugin.cpp)
target_include_directories(cpp_plugin PRIVATE 
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(cpp_plugin 
    ${CURL_LIBRARIES}
    jsoncpp_lib
)

# Set output properties
if(APPLE)
    set_target_properties(c_plugin PROPERTIES
        SUFFIX ".dylib"
        PREFIX ""
    )
    set_target_properties(cpp_plugin PROPERTIES
        SUFFIX ".dylib"
        PREFIX ""
    )
elseif(UNIX)
    set_target_properties(c_plugin PROPERTIES
        SUFFIX ".so"
        PREFIX ""
    )
    set_target_properties(cpp_plugin PROPERTIES
        SUFFIX ".so"
        PREFIX ""
    )
elseif(WIN32)
    set_target_properties(c_plugin PROPERTIES
        SUFFIX ".dll"
        PREFIX ""
    )
    set_target_properties(cpp_plugin PROPERTIES
        SUFFIX ".dll"
        PREFIX ""
    )
endif()

# Installation rules
install(TARGETS c_plugin cpp_plugin
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
)

install(FILES plugin_abi.h
    DESTINATION include
)

# Add custom target for testing
add_custom_target(test_info
    COMMAND ${CMAKE_COMMAND} -E echo "Testing plugins requires a running Play server at HOST=http://127.0.0.1:3000"
    COMMAND ${CMAKE_COMMAND} -E echo "Configure the plugins in your config.toml:"
    COMMAND ${CMAKE_COMMAND} -E echo "  [[plugin_config]]"
    COMMAND ${CMAKE_COMMAND} -E echo "  name = \"c_plugin\""
    COMMAND ${CMAKE_COMMAND} -E echo "  file_path = \"${CMAKE_CURRENT_BINARY_DIR}/c_plugin${CMAKE_SHARED_LIBRARY_SUFFIX}\""
    COMMAND ${CMAKE_COMMAND} -E echo "  url_prefix = \"/c-plugin\""
    COMMAND ${CMAKE_COMMAND} -E echo ""
    COMMAND ${CMAKE_COMMAND} -E echo "  [[plugin_config]]"
    COMMAND ${CMAKE_COMMAND} -E echo "  name = \"cpp_plugin\""
    COMMAND ${CMAKE_COMMAND} -E echo "  file_path = \"${CMAKE_CURRENT_BINARY_DIR}/cpp_plugin${CMAKE_SHARED_LIBRARY_SUFFIX}\""
    COMMAND ${CMAKE_COMMAND} -E echo "  url_prefix = \"/cpp-plugin\""
)