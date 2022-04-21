let test_dir = "./test"
let dottor_path = "../target/debug/dottor"

# build project
cargo build

# setup
rm -r $test_dir
mkdir $test_dir
cd $test_dir

../target/debug/dottor init
# setup example configs
../target/debug/dottor config oh-my-posh
../target/debug/dottor config nushell
