# Epoch Manager

The Epoch Manager is a contract which sole purpose is to create epochs on the Migaloo ecosystem.

## How it works

When the contract is instantiated, a start epoch and the epoch configuration are set up. The epoch configuration defines 
the duration of an epoch and when the genesis epoch is, i.e. the first epoch.

Once the genesis epoch is created, after the epoch duration has passed, anyone can create a new epoch by calling the 
`CreateEpoch` message. This action will create a new epoch, i.e. increase the epoch id by one, and alert the contracts 
that have registered for the hook.

## Epoch Hook

There are two actions that only the owner of the Epoch Manager can call for: `AddHook` and `RemoveHook`. These add a contract 
to the `HOOKS` list.

These contracts must implement the `EpochChangedHookMsg` interface, which is the signature of the message that will be 
executed on the hooks when a new epoch is created. The hook contains the current `Epoch`, specifying the id and start_time.
