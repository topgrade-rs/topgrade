#!/usr/bin/env sh
build_function() {
    rustup update
    cargo install cross


    echo -n "Building x86_64-linux-gnu"
    cross build --release --target x86_64-unknown-linux-gnu
    echo -n "Building x86_64-linux-musl"
    cross build --release --target x86_64-unknown-linux-musl
    echo -n "Building x86_64-freebsd"
    cross build --release --target x86_64-unknown-freebsd
    echo -n "Building aarch64-linux-gnu"
    cross build --release --target aarch64-unknown-linux-gnu
    echo -n "Building aarch64-linux-musl"
    cross build --release --target aarch64-unknown-linux-musl
    echo -n "Building x86_64-windows-gnu"
    cross build --release --target x86_64-pc-windows-gnu
}

package_function() {

    cd build
    mkdir x86_64-unknown-linux-gnu/
    mkdir x86_64-unknown-linux-musl/
    mkdir x86_64-unknown-freebsd/
    mkdir x86_64-pc-windows-gnu/
    mkdir aarch64-unknown-linux-gnu/
    mkdir aarch64-unknown-linux-musl/

    cp ../target/x86_64-unknown-linux-gnu/release/topgrade      x86_64-unknown-linux-gnu/
    cp ../target/x86_64-unknown-linux-musl/release/topgrade     x86_64-unknown-linux-musl/
    cp ../target/x86_64-unknown-freebsd/release/topgrade        x86_64-unknown-freebsd/topgrade
    cp ../target/aarch64-unknown-linux-gnu/release/topgrade     aarch64-unknown-linux-gnu/topgrade
    cp ../target/aarch64-unknown-linux-musl/release/topgrade    aarch64-unknown-linux-musl/topgrade
    cp ../target/x86_64-pc-windows-gnu/release/topgrade.exe     x86_64-pc-windows-gnu/topgrade.exe

    cd x86_64-unknown-linux-gnu/
    tar -czf ../topgrade-${ans}-x86_64-linux-gnu.tar.gz topgrade
    cd ..

    cd x86_64-unknown-linux-musl
    tar -czf ../topgrade-${ans}-x86_64-linux-musl.tar.gz  topgrade
    cd ..

    cd x86_64-unknown-freebsd/
    tar -czf ../topgrade-${ans}-x86_64-freebsd.tar.gz     topgrade
    cd ..

    cd aarch64-unknown-linux-gnu/
    tar -czf ../topgrade-${ans}-aarch64-linux-gnu.tar.gz  topgrade
    cd ..

    cd aarch64-unknown-linux-musl/
    tar -czf ../topgrade-${ans}-aarch64-linux-musl.tar.gz topgrade
    cd ..

    cd x86_64-pc-windows-gnu/
    zip -q ../topgrade-${ans}-x86_64-windows.zip           topgrade.exe
    cd ..
    cd ..

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
