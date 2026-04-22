rm -rf /Users/rajanpanneerselvam/work/AruviStudio/src-tauri/target/release/bundle/macos/AruviStudio.app

env MACOSX_DEPLOYMENT_TARGET=10.15 CMAKE_OSX_DEPLOYMENT_TARGET=10.15 CFLAGS=-mmacosx-version-min=10.15 CXXFLAGS=-mmacosx-version-min=10.15 npm run tauri build -- --bundles app

mkdir -p /Users/rajanpanneerselvam/work/releases && cp -R /Users/rajanpanneerselvam/work/AruviStudio/src-tauri/target/release/bundle/macos/AruviStudio.app /Users/rajanpanneerselvam/work/releases/
