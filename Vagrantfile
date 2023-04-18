Vagrant.configure("2") do |config|
  config.vm.box = "archlinux/archlinux"
  config.vm.provision "shell",
    inline: <<-EOS
      set -eo
      pacman -Syu --noconfirm rustup base-devel clang
      rustup default stable
      fallocate -l 128M /tmp/disk.img
      mv /tmp/disk.img /vagrant/
EOS
end
