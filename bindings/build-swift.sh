#!/bin/sh

set -eo pipefail

pushd `dirname $0`
trap popd EXIT

NAME="CooklangParser"
VERSION=${1:-"1.0"} # first arg or "1.0"
BUNDLE_IDENTIFIER="org.cooklang.$NAME"
LIBRARY_NAME="libcooklang_bindings.a"
FRAMEWORK_LIBRARY_NAME=${NAME}FFI
FRAMEWORK_NAME="$FRAMEWORK_LIBRARY_NAME.framework"
XC_FRAMEWORK_NAME="$FRAMEWORK_LIBRARY_NAME.xcframework"
HEADER_NAME="${NAME}FFI.h"
OUT_PATH="out"
MIN_IOS_VERSION="16.0"
WRAPPER_PATH="../swift/Sources/CooklangParser"

AARCH64_APPLE_IOS_PATH="../target/aarch64-apple-ios/release"
AARCH64_APPLE_IOS_SIM_PATH="../target/aarch64-apple-ios-sim/release"
AARCH64_APPLE_DARWIN_PATH="../target/aarch64-apple-darwin/release"
X86_64_APPLE_DARWIN_PATH="../target/x86_64-apple-darwin/release"

targets=("aarch64-apple-ios" "aarch64-apple-ios-sim" "aarch64-apple-darwin" "x86_64-apple-darwin")

# Build for all targets
for target in "${targets[@]}"; do
  echo "Building for $target..."
  rustup target add $target
  cargo build --release --target $target
done

# Generate swift wrapper
echo "Generating swift wrapper..."
mkdir -p $OUT_PATH
mkdir -p $WRAPPER_PATH
CURRENT_ARCH=$(rustc --version --verbose | grep host | cut -f2 -d' ')

cargo run --features="uniffi/cli"  \
      --bin uniffi-bindgen generate \
      --config uniffi.toml \
      --library ../target/$CURRENT_ARCH/release/$LIBRARY_NAME \
      --language swift \
      --out-dir $OUT_PATH

# Merge libraries with lipo
echo "Merging libraries with lipo..."
cp $AARCH64_APPLE_IOS_SIM_PATH/$LIBRARY_NAME $OUT_PATH/sim-$LIBRARY_NAME
lipo -create $AARCH64_APPLE_DARWIN_PATH/$LIBRARY_NAME \
             $X86_64_APPLE_DARWIN_PATH/$LIBRARY_NAME \
     -output $OUT_PATH/macos-$LIBRARY_NAME

# Create framework template
rm -rf $OUT_PATH/$FRAMEWORK_NAME
mkdir -p $OUT_PATH/$FRAMEWORK_NAME/Headers
mkdir -p $OUT_PATH/$FRAMEWORK_NAME/Modules
cp $OUT_PATH/$HEADER_NAME $OUT_PATH/$FRAMEWORK_NAME/Headers
cat <<EOT > $OUT_PATH/$FRAMEWORK_NAME/Modules/module.modulemap
framework module $FRAMEWORK_LIBRARY_NAME {
  umbrella header "$HEADER_NAME"

  export *
  module * { export * }
}
EOT

cat <<EOT > $OUT_PATH/$FRAMEWORK_NAME/Info.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>$FRAMEWORK_LIBRARY_NAME</string>
	<key>CFBundleIdentifier</key>
	<string>$BUNDLE_IDENTIFIER</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$FRAMEWORK_LIBRARY_NAME</string>
	<key>CFBundlePackageType</key>
	<string>FMWK</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>CFBundleVersion</key>
	<string>$VERSION</string>
	<key>NSPrincipalClass</key>
	<string></string>
	<key>MinimumOSVersion</key>
	<string>$MIN_IOS_VERSION</string>
</dict>
</plist>
EOT

# Prepare frameworks for each platform
rm -rf $OUT_PATH/frameworks
mkdir -p $OUT_PATH/frameworks/sim
mkdir -p $OUT_PATH/frameworks/ios
mkdir -p $OUT_PATH/frameworks/macos
cp -r $OUT_PATH/$FRAMEWORK_NAME $OUT_PATH/frameworks/sim/
cp -r $OUT_PATH/$FRAMEWORK_NAME $OUT_PATH/frameworks/ios/
cp -r $OUT_PATH/$FRAMEWORK_NAME $OUT_PATH/frameworks/macos/
mv $OUT_PATH/sim-$LIBRARY_NAME $OUT_PATH/frameworks/sim/$FRAMEWORK_NAME/$FRAMEWORK_LIBRARY_NAME
mv $OUT_PATH/macos-$LIBRARY_NAME $OUT_PATH/frameworks/macos/$FRAMEWORK_NAME/$FRAMEWORK_LIBRARY_NAME
cp $AARCH64_APPLE_IOS_PATH/$LIBRARY_NAME $OUT_PATH/frameworks/ios/$FRAMEWORK_NAME/$FRAMEWORK_LIBRARY_NAME

# Convert macOS framework to versioned bundle structure
echo "Converting macOS framework to versioned bundle structure..."
MACOS_FRAMEWORK_PATH="$OUT_PATH/frameworks/macos/$FRAMEWORK_NAME"
mkdir -p "$MACOS_FRAMEWORK_PATH/Versions/A/Headers"
mkdir -p "$MACOS_FRAMEWORK_PATH/Versions/A/Modules"
mkdir -p "$MACOS_FRAMEWORK_PATH/Versions/A/Resources"
mv "$MACOS_FRAMEWORK_PATH/$FRAMEWORK_LIBRARY_NAME" "$MACOS_FRAMEWORK_PATH/Versions/A/$FRAMEWORK_LIBRARY_NAME"
mv "$MACOS_FRAMEWORK_PATH/Headers/$HEADER_NAME" "$MACOS_FRAMEWORK_PATH/Versions/A/Headers/$HEADER_NAME"
mv "$MACOS_FRAMEWORK_PATH/Modules/module.modulemap" "$MACOS_FRAMEWORK_PATH/Versions/A/Modules/module.modulemap"
mv "$MACOS_FRAMEWORK_PATH/Info.plist" "$MACOS_FRAMEWORK_PATH/Versions/A/Resources/Info.plist"
rmdir "$MACOS_FRAMEWORK_PATH/Headers"
rmdir "$MACOS_FRAMEWORK_PATH/Modules"
ln -s A "$MACOS_FRAMEWORK_PATH/Versions/Current"
ln -s Versions/Current/$FRAMEWORK_LIBRARY_NAME "$MACOS_FRAMEWORK_PATH/$FRAMEWORK_LIBRARY_NAME"
ln -s Versions/Current/Headers "$MACOS_FRAMEWORK_PATH/Headers"
ln -s Versions/Current/Modules "$MACOS_FRAMEWORK_PATH/Modules"
ln -s Versions/Current/Resources "$MACOS_FRAMEWORK_PATH/Resources"

# Create xcframework
echo "Creating xcframework..."
rm -rf $OUT_PATH/$XC_FRAMEWORK_NAME
xcodebuild -create-xcframework \
    -framework $OUT_PATH/frameworks/sim/$FRAMEWORK_NAME \
    -framework $OUT_PATH/frameworks/ios/$FRAMEWORK_NAME \
    -framework $OUT_PATH/frameworks/macos/$FRAMEWORK_NAME \
    -output $OUT_PATH/$XC_FRAMEWORK_NAME

# Copy swift wrapper
# Need some temporary workarounds to compile swift wrapper
# https://github.com/rust-lang/cargo/issues/11953
cat <<EOT > $OUT_PATH/import.txt
#if os(macOS)
import SystemConfiguration
#endif
EOT
cat $OUT_PATH/import.txt $OUT_PATH/$NAME.swift > $WRAPPER_PATH/$NAME.swift

# Create zip archive
echo "Creating zip archive..."
ZIP_NAME="$XC_FRAMEWORK_NAME.zip"
pushd $OUT_PATH
rm -rf $ZIP_NAME
zip -r $ZIP_NAME $XC_FRAMEWORK_NAME
popd

# Calculate SHA256 checksum
echo "Calculating SHA256 checksum..."
SHA256=$(shasum -a 256 $OUT_PATH/$ZIP_NAME | awk '{print $1}')
echo "SHA256: $SHA256"

# Update Package.swift with new version and checksum
echo "Updating Package.swift..."
PACKAGE_SWIFT_PATH="../Package.swift"
sed -i '' "s|url: \"https://github.com/cooklang/cooklang-rs/releases/download/v[^/]*/CooklangParserFFI.xcframework.zip\"|url: \"https://github.com/cooklang/cooklang-rs/releases/download/v$VERSION/CooklangParserFFI.xcframework.zip\"|" $PACKAGE_SWIFT_PATH
sed -i '' "s|checksum: \"[^\"]*\"|checksum: \"$SHA256\"|" $PACKAGE_SWIFT_PATH

echo "Build complete! Archive ready at: $OUT_PATH/$ZIP_NAME"
echo "Version: $VERSION"
echo "SHA256: $SHA256"
