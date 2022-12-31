if exists(":NeoBundleUpdate")
    echo "NeoBundle"
    NeoBundleUpdate
endif

if exists(":PluginUpdate")
    echo "Plugin"
    PluginUpdate
endif

if exists(":PlugUpgrade")
    echo "Plug"
    PlugUpgrade
    if $TOPGRADE_FORCE_PLUGUPDATE
        PlugUpdate!
    else
        PlugUpdate
    endif
endif

if exists("*dein#update()")
    echo "dein#update()"
    call dein#update()
endif

if exists(":DeinUpdate")
    echo "DeinUpdate"
    DeinUpdate
endif

if exists(":PaqUpdate")
    echo "PaqUpdate"
    PaqUpdate
endif

if exists(":CocUpdateSync")
    echo "CocUpdateSync"
    CocUpdateSync
endif

if exists(":Lazy")
    echo "Lazy"
    Lazy update
endif

" TODO: Should this be after `PackerSync`?
" Not sure how to sequence this after Packer without doing something weird
" with that `PackerComplete` autocommand.
if exists(":TSUpdate")
    echo "TreeSitter Update"
    TSUpdate
endif

if exists(':PackerSync')
  echo "Packer"
  autocmd User PackerComplete quitall
  PackerSync
else
  quitall
endif
