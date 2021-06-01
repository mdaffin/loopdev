Vagrant.configure("2") do |config|
  config.vm.box = "ubuntu/bionic64"
  config.vm.provision "shell",
    inline: <<-EOS
      curl https://sh.rustup.rs -sSf | sh -s -- -y
      apt install -y gcc
      fallocate -l 128M /tmp/disk.img
      mv /tmp/disk.img /vagrant/
EOS
end
