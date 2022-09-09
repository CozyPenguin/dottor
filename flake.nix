{
  description = "Flake for dottor";

  outputs = { self, nixpkgs, flake-utils }: {
    overlay = final: prev: {
      dottor = self.packages."${prev.system}".dottor;
    };
  } // (flake-utils.lib.eachDefaultSystem (system:
    let pkgs = import nixpkgs { inherit system; }; in rec {
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
      defaultApp = apps.dottor;
    }
  ));
}
