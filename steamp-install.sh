#!/bin/bash

# 获取最新版本信息
response=$(curl -s https://api.github.com/repos/kamibook/steamp/releases/latest)
version=$(echo "$response" | grep 'tag_name' | cut -d'"' -f4)

# 获取系统架构
arch=$(uname -m)

# 安装或更新 steamp 的函数
install_or_update_steamp() {
    local action="$1"
    if command -v dpkg &> /dev/null; then
        wget https://github.com/kamibook/steamp/releases/download/$version/steamp-$version-1.$arch.deb
        sudo dpkg -i steamp-$version-1.$arch.deb
        rm steamp-$version-1.$arch.deb
        echo "steamp-$version-1.$arch.deb $action successfully"
    elif command -v rpm &> /dev/null; then
        wget https://github.com/kamibook/steamp/releases/download/$version/steamp-$version-1.$arch.rpm
        sudo rpm -ivh steamp-$version-1.$arch.rpm
        rm steamp-$version-1.$arch.rpm
        echo "steamp-$version-1.$arch.rpm $action successfully"
    else
        echo "Unsupported package manager"
    fi
}

# 检查是否安装了 steamp，如已安装则检查是否需要更新。
if command -v steamp &> /dev/null; then
    echo "steamp is already installed"
    current_version=$(steamp --version | awk '{print $2}')
    echo "Current version: $current_version"
    echo "Latest version: $version"

    function version_lt() { test "$(echo "$@" | tr " " "\n" | sort -rV | head -n 1)" != "$1"; }
 
    if version_lt "$current_version" "$version"; then
        echo "steamp is not up to date"
        install_or_update_steamp "updated"
    else
        echo "steamp is up to date"
    fi
else
    echo "steamp is not installed"
    install_or_update_steamp "installed"
fi
