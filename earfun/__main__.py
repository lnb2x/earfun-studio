import argparse
import sys


def main():
    parser = argparse.ArgumentParser(description='EarFun Studio cho Ubuntu / TW600')
    parser.add_argument('--smoke-test', action='store_true', help='Mở và kiểm tra sáu trang, không truy cập Bluetooth')
    parser.add_argument('--connect-last', action='store_true', help='Kết nối thiết bị cuối sau khi mở cửa sổ')
    args = parser.parse_args()
    from .ui import App
    return App(smoke=args.smoke_test, connect_last=args.connect_last).run([sys.argv[0]])


if __name__ == '__main__':
    sys.exit(main())
