If you want to set up multicast as a service that is run at startup,
you can use the `multicast.service` script in this directory.

Make sure to install the binary at `/usr/local/bin/multicast`, or
adjust the `multicast.service` file.

Tested on Ubuntu 22.04:

```bash
sudo cp ./multicast.service /etc/systemd/system
sudo systemctl start multicast
# Test if the service runs and replies
# If it does:
sudo systemctl enable multicast
```
