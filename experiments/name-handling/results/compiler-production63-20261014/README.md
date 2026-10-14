# Compiler consumer build capture

The current-source build stops on a double Arc wrapper in a newly added taint test fixture. No semantic controls execute; source subset drift is zero. The fixture is corrected to clone the actual CommandRegistry before wrapping it. Original assertions, plan, logs and receipts remain unchanged.
