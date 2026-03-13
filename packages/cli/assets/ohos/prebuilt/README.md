This folder contains prebuilt OpenSSL bundles for OpenHarmony builds.

The archive is structured similarly to the Android prebuilt OpenSSL bundle used by `dx`:

- `ssl/include`: OpenSSL headers
- `ssl/libs/ohos.arm64-v8a`: arm64 shared libraries
- `ssl/libs/ohos.armeabi-v7a`: armv7 shared libraries
- `ssl/libs/ohos.x86_64`: x86_64 shared libraries

`dx` unpacks this archive into `~/.dx/prebuilt/openssl-3.4.0-dev-ohos-1` and uses it as the
default OpenSSL location for OpenHarmony builds and bundle-time dependency copying.
