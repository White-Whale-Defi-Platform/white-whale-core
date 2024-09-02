#!/usr/bin/env bash

## Loads the base env for a given chain. i.e. the TXFLAG variable.

b_flag=sync
output_flag=json

case $chain in

local | juno | juno-testnet | terra | terra-testnet | comdex | comdex-testnet | sei | sei-testnet | stargaze-testnet | osmosis | osmosis-testnet | orai)
	if [ -n "$ZSH_VERSION" ]; then
		# Using an array for TXFLAG
		TXFLAG=(--node $RPC --chain-id $CHAIN_ID --gas-prices 0.25$DENOM --gas auto --gas-adjustment 1.4 -y -b $b_flag --output $output_flag)
	else
		# Using a string for TXFLAG
		TXFLAG="--node $RPC --chain-id $CHAIN_ID --gas-prices 0.25$DENOM --gas auto --gas-adjustment 1.4 -y -b $b_flag --output $output_flag"
	fi
	;;

chihuahua | migaloo | migaloo-testnet)
	if [ -n "$ZSH_VERSION" ]; then
		# Using an array for TXFLAG
		TXFLAG=(--node $RPC --chain-id $CHAIN_ID --gas-prices 1$DENOM --gas auto --gas-adjustment 1.4 -y -b $b_flag --output $output_flag)
	else
		# Using a string for TXFLAG
		TXFLAG="--node $RPC --chain-id $CHAIN_ID --gas-prices 1$DENOM --gas auto --gas-adjustment 1.4 -y -b $b_flag --output $output_flag"
	fi
	;;

injective | injective-testnet)
	if [ -n "$ZSH_VERSION" ]; then
		# Using an array for TXFLAG
		TXFLAG=(--node $RPC --chain-id $CHAIN_ID --gas-prices=500000000inj --gas 10000000 -y -b $b_flag --output $output_flag)
	else
		# Using a string for TXFLAG
		TXFLAG="--node $RPC --chain-id $CHAIN_ID --gas-prices=500000000inj --gas 10000000 -y -b $b_flag --output $output_flag"
	fi
	;;

archway | archway-testnet)
	if [ -n "$ZSH_VERSION" ]; then
		# Using an array for TXFLAG
		TXFLAG=(--node $RPC --chain-id $CHAIN_ID --gas-prices 140000000000.000000000000000000$DENOM --gas auto --gas-adjustment 1.4 -y -b $b_flag --output $output_flag)
	else
		# Using a string for TXFLAG
		TXFLAG="--node $RPC --chain-id $CHAIN_ID --gas-prices 140000000000.000000000000000000$DENOM --gas auto --gas-adjustment 1.4 -y -b $b_flag --output $output_flag"
	fi
	;;

*)
	echo "Network $chain not defined"
	return 1
	;;
esac

export TXFLAG
