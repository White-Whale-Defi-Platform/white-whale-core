import json
import argparse

def generate_json(start_time, num_periods, period_duration, total_coins):
    base_coins = total_coins // num_periods
    extra_coins = total_coins % num_periods

    periods = [
        {
            "coins": f"{base_coins + (1 if i < extra_coins else 0)}uwhale",
            "length_seconds": period_duration
        }
        for i in range(num_periods)
    ]

    output_json = {
        "start_time": start_time,
        "periods": periods
    }

    return json.dumps(output_json, indent=2)

def save_json_to_file(json_data, filename):
    with open(filename, "w") as file:
        file.write(json_data)

def verify_json(json_data, expected_periods, expected_total_coins):
    data = json.loads(json_data)
    num_periods = len(data['periods'])

    if num_periods != expected_periods:
        print(f"Error: Expected {expected_periods} periods, but found {num_periods}")
        return False

    total_coins = sum(int(period['coins'].rstrip('uwhale')) for period in data['periods'])

    if total_coins != expected_total_coins:
        print(f"Error: Expected {expected_total_coins} total coins, but found {total_coins}")
        return False

    print("Verification successful: The number of periods and total coins match the expected values.")
    return True

def parse_arguments():
    parser = argparse.ArgumentParser(description="Generate a JSON file with given parameters.")
    parser.add_argument("start_time", type=int, help="Start time (Unix timestamp).")
    parser.add_argument("num_periods", type=int, help="Number of periods.")
    parser.add_argument("period_duration", type=int, help="Duration of each period in seconds.")
    parser.add_argument("total_coins", type=int, help="Total amount of coins to distribute.")
    parser.add_argument("filename", type=str, help="Output filename for the generated JSON file.")

    return parser.parse_args()

if __name__ == "__main__":
    args = parse_arguments()
    result_json = generate_json(args.start_time, args.num_periods, args.period_duration, args.total_coins)
    
    if verify_json(result_json, args.num_periods, args.total_coins):
        save_json_to_file(result_json, args.filename)
        print(f"JSON data saved to {args.filename}")
    else:
        print("Verification failed. JSON data not saved.")

