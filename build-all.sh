#!/usr/bin/env sh
build_function() {
    rustup update
    cargo install cross


    cross build --release --target x86_64-unknown-linux-gnu
    cross build --release --target x86_64-unknown-linux-musl
    cross build --release --target x86_64-unknown-freebsd
    cross build --release --target aarch64-unknown-linux-gnu
    cross build --release --target aarch64-unknown-linux-musl
    cross build --release --target x86_64-pc-windows-gnu
}

package_function() {
    tar -czvf build/topgrade-${ans}-x86_64-linux-gnu.tar.gz target/x86_64-unknown-linux-gnu/release/topgrade-rs
    tar -czvf build/topgrade-${ans}-x86_64-linux-musl.tar.gz target/x86_64-unknown-linux-musl/release/topgrade-rs
    tar -czvf build/topgrade-${ans}-x86_64-freebsd.tar.gz target/x86_64-unknown-freebsd/release/topgrade-rs
    tar -czvf build/topgrade-${ans}-aarch64-linux-gnu.tar.gz target/aarch64-unknown-linux-gnu/release/topgrade-rs
    tar -czvf build/topgrade-${ans}-aarch64-linux-musl.tar.gz target/aarch64-unknown-linux-musl/release/topgrade-rs
    zip -q build/topgrade-${ans}-x86_64-windows.zip target/x86_64-pc-windows-gnu/release/topgrade-rs.exe


}

print_checksums() {


    cd build/
    sha256sum topgrade-${ans}-*
    cd ../
}

while true; do

echo "You should always have a look on scripts you download from the internet."
read -p "Do you still want to proceed? (y/n) " yn

echo -n "Input version number: "
read ans
mkdir build

case $yn in
	y ) build_function
        package_function
        print_checksums
		break;;
	n ) echo exiting...;
		exit;;
	* ) echo invalid response;;
esac

done
