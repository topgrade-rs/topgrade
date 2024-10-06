# Containers step

* New default behavior: In the previous versions, if you have both Docker and 
  Podman installed, Podman will be used by Topgrade. Now the default option
  has been changed to Docker. This can be overridden by setting the 
  `container.runtime` option in the configuration TOML to "podman".
