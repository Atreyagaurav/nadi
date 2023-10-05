# Maintainer: Gaurav Atreya <allmanpride@gmail.com>
pkgname=nadi
pkgver=0.1.5
pkgrel=1
pkgdesc="Not Available Data Integration"
arch=('x86_64')
license=('GPL3')
depends=('gcc-libs' 'gdal')
makedepends=('rust' 'cargo')

build() {
	cargo build --release
}

package() {
    cd "$srcdir"
    mkdir -p "$pkgdir/usr/bin"
    cp "../target/release/${pkgname}" "$pkgdir/usr/bin/${pkgname}"
}
