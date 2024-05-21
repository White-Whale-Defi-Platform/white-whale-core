# Vault Manager

The Vault Manager is a contract handling a collection of single asset vaults that bots use to access flashloan capital for 
arbitrage, liquidation, and other DeFi uses. By using the Flash Loan Vaults, arbitrage and liquidations happen locally 
in a capital-efficient manner and without capital requirements. That is, each arbitrageur or liquidator will no longer 
need their own capital on each local chain waiting idly to arb or to liquidate because they can access a flash loan for 
their capital needs. When an arbitrage opportunity arises, an arbitrageur takes a flash loan, arbs the local dex price 
versus a Migaloo pool, and then pays back the loan plus the flash loan fee. The arbitrageur then keeps the profit without 
having used any of their own capital. 

Depositors of tokens into flash loan vaults benefit from fees paid when their vault is accessed for flash loans; the 
greater the volume, the more fees generated. Flash loan vaults are a great source of yield with no impermanent loss.

## How it works

The following is a high-level overview of how the Vault Manager works. It touches on some technical details, assisting
developers in understanding the contract's inner workings, while also providing a general understanding of the contract's
functionality, so a regular user can understand how to interact with it.

### Vault Creation

Creating vaults is a simple and permissionless process. A user can call the `CreateVault` message, with the desired vault
parameters together with the vault creation fee. The vault creation fee is a protocol fee that is sent to the Bonding
Manager.

Vaults are distinguished by unique IDs, meaning there can be multiple vaults for the same asset, e.i. multiple whale
vaults, as long as they have different identifiers. Vaults cannot be removed or updated once created, so it is important
to get the parameters right from the start.

### Deposits

Users can deposit into a vault at any time by calling the `Deposit` message, containing the vault identifier they wish to 
deposit assets into. Each vault is stored in the state of the contract, and it keeps track of the assets deposited in it. 

When a user deposits into a vault, the assets are transferred to the vault's balance, while the user receives an LP token 
representing the share of the vault they own. The LP token can be used to withdraw the assets from the vault at any time.

### Withdrawals

Users can withdraw from a vault at any time by calling the `Withdraw` message, sending the amount of LP tokens they got 
when depositing assets into a vault. Since each `Vault` has a unique LP token there's no need to specify a vault 
identifier when withdrawing as the contract can find the Vault the user deposited into by analyzing the LP token sent. 

The LP tokens are burned and the original assets are transferred back to the user.

### Flash loans

Flash loans are a powerful tool that allows bots and users to borrow assets from the vault without any collateral, with 
the condition that the assets plus the flash loan fees are returned within the same transaction, otherwise the transaction 
is reverted as if nothing had happened. 

A flash loan can be taken by calling the `FlashLoan` message, with the desired amount, vault identifier and the payload. 
The payload is a list of messages that will be executed in the same transaction, and it doesn't need to include the "payback" 
transaction as it is handled by the contract.

When a flash loan is taken, a boolean in the state is set to true on `ONGOING_FLASHLOAN`, so the funds can't be used to 
be deposited back into a vault or to take another flash loan. After the payload is executed, the `CallbackMsg::AfterFlashloan` 
is called. This makes sure the funds are back in the vault plus the fees. The profit made from the payload operations is 
sent back to the originator of the flash loan. The Bonding Manager receives the protocol fees via the `FillRewards` message 
and the users that have deposited into the vault where the flash loan was taken from receive the flash loan fees.
