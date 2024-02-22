{
  description = "Flake for dottor";

  inputs = {
    nixpkgs.url = github:nixos/nixpkgs;
    fenix = {
      url = github:nix-community/fenix;
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = github:numtide/flake-utils;
  };

  outputs = { self, nixpkgs, fenix, flake-utils }: {
    overlay = final: prev: {
      dottor = self.packages."${prev.system}".dottor;
    };
  } // (flake-utils.lib.eachDefaultSystem (system:
    let pkgs = import nixpkgs { inherit system; overlays = [ fenix.overlays.default ]; }; in rec {
      packages.dottor = pkgs.rustPlatform.buildRustPackage rec {
        pname = "dottor";
        version = "0.1.0";
      
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.openssl ];
      
        meta = with pkgs.lib; {
          description = "A simple dotfiles manager";
          homepage = "https://github.com/CozyPenguin/dottor";
          license = licenses.mit;
          mainProgram = "dottor";
        };
      };
      defaultPackage = packages.dottor;
      apps.dottor = flake-utils.lib.mkApp { drv = packages.dottor; };
      apps.default = apps.dottor;

      devShell = pkgs.mkShell {
        packages = [ 
          (pkgs.fenix.complete.withComponents [
            "cargo"
            "rust-src"
            "clippy"
            "rustc"
            "rustfmt"
          ])
          pkgs.rust-analyzer-nightly
        ];
        inputsFrom = [
          packages.dottor
        ];
      };
    }
  ));
}
