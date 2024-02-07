#!/bin/bash
export PATH=$PATH:/usr/local/bin

#
# White Whale contracts pre-commit hook, used to perform static analysis checks on changed files.
#
# Install the hook with the --install option.
#

project_toplevel=$(git rev-parse --show-toplevel)
git_directory=$(git rev-parse --git-dir)

install_hook() {
	mkdir -p "$git_directory/hooks"
	ln -sfv "$project_toplevel/scripts/hooks/pre-commit.sh" "$git_directory/hooks/pre-commit"
	cargo install taplo-cli --locked
}

if [ "$1" = "--install" ]; then
	if [ -f "$git_directory/hooks/pre-commit" ]; then
		read -r -p "There's an existing pre-commit hook. Do you want to overwrite it? [y/N] " response
		case "$response" in
		[yY][eE][sS] | [yY])
			install_hook
			;;
		*)
			printf "Skipping hook installation :("
			exit $?
			;;
		esac
	else
		install_hook
	fi
	exit $?
fi

# cargo fmt checks
format_check() {
	printf "Starting file formatting check...\n"

	has_formatting_issues=0
	first_file=1

	# Check and format Rust files
	rust_staged_files=$(git diff --name-only --staged -- '*.rs')
	format_files "$rust_staged_files"

	# Check and format TOML files
	toml_staged_files=$(git diff --name-only --staged -- '*.toml')
	format_files "$toml_staged_files"

	# Check and format Shell script files
	sh_staged_files=$(git diff --name-only --staged -- '*.sh')
	format_files "$sh_staged_files"

	if [ $has_formatting_issues -ne 0 ]; then
		printf "\nSome files were formatted and added to the commit.\n"
	fi
}

format_files() {
	local staged_files=$1

	for file in $staged_files; do
		case "$file" in
		*.rs)
			format_check_result=$(rustfmt --check $file 2>&1)
			;;
		*.toml)
			taplo fmt $file >/dev/null 2>&1
			git add $file
			printf "$file\n"
			;;
		*.sh)
			format_check_result=$(shfmt -d $file 2>&1)
			;;
		esac

		format_file
	done
}

format_file() {
	if [ "$format_check_result" != "" ]; then
		if [ $first_file -eq 0 ]; then
			printf "\n"
		fi
		printf "$file"
		has_formatting_issues=1
		first_file=0

		case "$file" in
		*.rs)
			rustfmt $file
			;;
		*.sh)
			shfmt -w $file
			;;
		esac

		# Add formatted file to commit if changes were made
		not_staged_file=$(git diff --name-only -- $file)
		if [ "$not_staged_file" != "" ]; then
			git add $not_staged_file
		fi
	fi
}

# clippy checks
lint_check() {
	printf "Starting clippy check...\n"
	RUSTFLAGS="-Dwarnings"
	cargo clippy --quiet -- -D warnings
	clippy_exit_code=$?
	if [ $clippy_exit_code -ne 0 ]; then
		printf "\nclippy found some issues. Fix them manually and try again :)"
		exit 1
	fi
}

format_check
lint_check
