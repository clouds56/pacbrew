post_install() {
  mkdir -p $PREFIX/etc/ca-certificates
  /usr/bin/security find-certificate -a -p /Library/Keychains/System.keychain /System/Library/Keychains/SystemRootCertificates.keychain > $PREFIX/etc/ca-certificates/cert.pem
}
