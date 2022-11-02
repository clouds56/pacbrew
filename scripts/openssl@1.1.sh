post_install() {
  mkdir $PREFIX/etc/$PKG_NAME
  ln -s ../ca-certificates/cert.pem $PREFIX/etc/$PKG_NAME/cert.pem
}
