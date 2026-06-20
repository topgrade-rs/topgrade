(when (featurep 'package)
  (if (fboundp 'package-upgrade-all)
    (package-upgrade-all nil)
    (message "Your Emacs version doesn't support unattended packages upgrade")))
