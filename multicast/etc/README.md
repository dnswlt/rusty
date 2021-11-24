If you want to set up multicast as a service that is run at startup,
you can use the `multicast.service` script in this directory.

Test it:

```
sudo cp ./multicast.service /etc/systemd/system
sudo servicectl start multicast
# Test if the service runs and replies
# If it does:
sudo servicectl enable multicast
```

