# Hardware testing checklist

Everything below needs a person with a real trainer. Record results in the PR or in a compatibility issue.
Items marked `HW-VERIFY` in the code are listed here.

## Spike (before trusting any control code)

- [ ] Explore the trainer with a generic BLE tool (for example nRF Connect). Record the services it exposes
      (FTMS 0x1826, Cycling Power 0x1818, CSC 0x1816, vendor services) and the maximum number of simultaneous connections.
- [ ] `erglite-cli scan --features ble` lists the trainer and the heart rate monitor.
- [ ] Capture raw Indoor Bike Data frames at rest, at 100 W and at 200 W; add them to `fixtures/ble-traces/`.
- [ ] Read Fitness Machine Feature and Supported Power Range; compare with the manufacturer datasheet.

## ERG

- [ ] Request Control returns success (`0x80 0x00 0x01`).
- [ ] A target power change reaches the trainer and the measured power converges. Note the delay.
- [ ] Ramps: targets sent once per second do not cause errors or oscillation.
- [ ] Pause and resume behave as expected; the 50 W pause target is acceptable.
- [ ] Targets above the trainer maximum are rejected or clipped (`HW-VERIFY` in `ble.rs`).

## Grade simulation

- [ ] `0x11` is accepted; status notification `0x12` arrives.
- [ ] Characterize the feel: steady power at fixed cadence with grades of 0, 3, 6 and 9 %.
- [ ] Decide the mass correction factor (FTMS does not carry the rider mass) and update the profile defaults.
- [ ] Check whether the trainer-reported speed is reliable in simulation mode; otherwise use the app physics model.
- [ ] Maximum grade the trainer supports.

## Robustness

- [ ] Disconnect and reconnect during a session: the session pauses and resumes.
- [ ] A 2 hour session with no data loss, on Windows, macOS and Linux.
- [ ] Trainer connected to another app: the error message is clear.
