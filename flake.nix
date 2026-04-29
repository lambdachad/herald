{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
        cuda = pkgs.cudaPackages_13;

        onnxruntime-gpu = pkgs.fetchurl {
          url = "https://github.com/microsoft/onnxruntime/releases/download/v1.25.1/onnxruntime-linux-x64-gpu_cuda13-1.25.1.tgz";
          hash = "sha256-68FOEpDbKjCnu0Fb0cPhOQp4FrtNuHZ33DbQce0igzw=";
        };

        ort-lib = pkgs.runCommand "ort-lib-1.25.1" {} ''
          mkdir -p $out/lib
          tar xzf ${onnxruntime-gpu} --strip-components=2 -C $out/lib \
            onnxruntime-linux-x64-gpu-1.25.1/lib/
        '';
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            pkg-config
            alsa-lib
            openssl
            cuda.cudatoolkit
            cuda.cudnn
          ];

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          CUDA_HOME = "${cuda.cudatoolkit}";
          ORT_DYLIB_PATH = "${ort-lib}/lib/libonnxruntime.so";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.alsa-lib
            cuda.cudatoolkit
            cuda.cudnn.lib
            "${ort-lib}"
            "/run/opengl-driver"
          ];
        };
      });
}
