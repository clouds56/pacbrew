post_install() {
  mkdir -p $PREFIX/lib/node_modules
  rm -rf $PREFIX/lib/node_modules/npm
  cp -r $CELLAR/libexec/lib/node_modules/npm $PREFIX/lib/node_modules/npm
  ln -sf $PREFIX/lib/node_modules/npm/bin/npm-cli.js $PREFIX/bin/npm
  ln -sf $PREFIX/lib/node_modules/npm/bin/npx-cli.js $PREFIX/bin/npx

  echo "prefix = $PREFIX\n" > $PREFIX/lib/node_modules/npm/npmrc
}
