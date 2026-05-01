# Tech Stack

## Overview
Tauri v2 桌面应用，Rust 后端处理 Bethesda 游戏文件解析与翻译，TypeScript + React 前端提供虚拟化表格 UI。

## Languages
**Rust** (backend), **TypeScript** (frontend)

Rust 用于 ESP/BSA 二进制解析、缓存、翻译 API 调用等高性能 I/O 操作。TypeScript 提供类型安全的 React UI。

## Framework
**Tauri v2** + **React** + **Vite**

Tauri v2 提供原生桌面壳与 Rust→前端 IPC。React + Vite 提供快速开发体验和虚拟化表格渲染（react-window）。

## Authentication
**无需用户认证**

翻译工具本地运行，API Key（OpenAI/DeepL）存储在内存中，不持久化。

## Infrastructure & Deployment
**Tauri 自动更新 + GitHub Releases**

通过 Tauri updater 插件从 GitHub Releases 拉取更新。Windows 为主要目标平台。

## Package Manager
**Cargo** (Rust), **npm** (TypeScript)

Cargo workspace 包含 4 个 crate（xt-core, xt-shared, xt-cli, xtranslator-tauri）。前端使用 npm 管理依赖。
