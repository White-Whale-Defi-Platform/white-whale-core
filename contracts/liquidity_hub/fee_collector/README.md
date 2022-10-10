# Fee Collector

The Fee Collector is responsible for collecting the protocol fees from the WW pools and flash loan vaults on the Liquidity Hub.
There's a single Fee Collector per Liquidity Hub, though they are all connected to the Interchain Fee Collector, which collects
the protocol fees on each Fee Collector no matter the blockchain they live on.

The WW pools accrue protocol fees when swapping assets, while the vaults do it when flash loans are taken.

The protocol fees are kept on the WW pools and vaults respectively. At any point, a message to collect the protocol fees
can be sent directly to the pools or vaults. In that case, the fees that have been collected so far by the given contract will be
sent to the Fee Collector.

Alternatively, the protocol fee collection mechanism can be triggered via the Fee Collector by using the message `CollectFees`,
using the desired `CollectFeesFor` parameter. This allows the Fee Collector to collect the protocol fees for specific contracts
or for all the contracts created by a Factory (i.e. WW pools or vaults).
