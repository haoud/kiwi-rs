<!-- Improved compatibility of back to top link: See: https://github.com/othneildrew/Best-README-Template/pull/73 -->
<a name="readme-top"></a>
<!--
*** Thanks for checking out the Best-README-Template. If you have a suggestion
*** that would make this better, please fork the repo and create a pull request
*** or simply open an issue with the tag "enhancement".
*** Don't forget to give the project a star!
*** Thanks again! Now go create something AMAZING! :D
-->

<!-- PROJECT LOGO -->
<br />
<!--<div align="center">
  <a href="https://github.com/haoud/silicium">
    <img src="images/logo.png" alt="Logo" width="80" height="80">
  </a>-->

<!--<h3 align="center">Silicium</h3>-->
<h1 align="center">Kiwi</h1>
  <p align="center">
    A simple and educative micro-kernel written in Rust trying to explore modern concepts
    <br />
    <a href="https://github.com/haoud/kiwi"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="https://github.com/haoud/kiwi">View Demo</a>
    ·
    <a href="https://github.com/haoud/kiwi/issues">Report Bug</a>
    ·
    <a href="https://github.com/haoud/kiwi/issues">Request Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#building">Building</a></li>
      </ul>
    </li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

TODO

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

To build this project, you will need to have Rust **and** `rustup` installed on your machine. You can install it by following the instructions on the [official website](https://www.rust-lang.org/tools/install).

There are also a few more dependencies in order to build and run the project:
- `qemu` for running the kernel in a virtual machine. You can install it with your package manager. Make sure to install the version corresponding to your target architecture (e.g. `qemu-system-riscv64` if you want to run the riscv64 kernel).

### Building

Clone the repository:
```sh
git clone --depth 1 https://github.com/haoud/silicium.git
```
Build the kernel, servers and userland programs:
```sh
make build
```
Run the kernel in Qemy:
```sh
make run
```

> [!TIP]
> If you are lost, you can run `make help` to see all the available commands.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Kiwi is dual-licensed under the Apache License, Version 2.0 and the MIT license.
See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

* [README Template](https://github.com/othneildrew/Best-README-Template/blob/master/README.md)

<p align="right">(<a href="#readme-top">back to top</a>)</p>
