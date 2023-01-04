self: super: {
  stdenv =
    with super;(overrideCC stdenv gcc9).override
      { cc = super.stdenv.cc; };
}
#  prev.stdenv.mkDerivation {
#  name = "merlinToolchain"
#    src = fetchgit {
#  url =
#  "https://git.yoctoproject.org/opkg-utils";
#  sha256 = "kO4mUJKE6vtiOIvCiMcYo+5UuoL8AzpmT1hluHrlafg=";
#};



