print("Hello, Python!")

import sys

def main():
    print(f"sys.argv: {sys.argv}")

    if len(sys.argv) > 1:
        code = sys.argv[1]  # CMD で渡されたコード
        try:
            exec(code)  # Python コードを実行
        except Exception as e:
            print(f"Error: {e}")
    else:
        print("No code provided.")

if __name__ == "__main__":
    main()
