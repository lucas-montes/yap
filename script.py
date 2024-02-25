import argparse

def _get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--num-files",
        type=int,
        required=True,
    )
    parser.add_argument(
        "--zone",
        type=str,
        required=True,
    )
    args = parser.parse_args()
    return args


def main():
    args = _get_args()
    print("doing python things")
    if args.num_files == 2:
        with open("third_out.txt", "a+") as f:
            f.write(args.zone)
        with open("second_out.txt", "a+") as f:
            f.write(args.zone)
    else:
        with open("first_out.txt", "a+") as f:
            f.write(args.zone)

if __name__ == "__main__":
    main()
