import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../util/extension/build_context_extension.dart';
import '../../wallet_constants.dart';
import '../card/overview/card_overview_screen.dart';
import '../menu/menu_screen.dart';
import '../qr/qr_screen.dart';
import 'bloc/home_bloc.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: _buildBody(),
      bottomNavigationBar: _buildBottomNavigationBar(context),
    );
  }

  Widget _buildBody() {
    return BlocBuilder<HomeBloc, HomeState>(
      builder: (context, state) {
        final Widget tab;
        switch (state.tab) {
          case HomeTab.cards:
            tab = const CardOverviewScreen();
            break;
          case HomeTab.qr:
            tab = const QrScreen();
            break;
          case HomeTab.menu:
            tab = const MenuScreen();
            break;
        }
        return SafeArea(child: tab);
      },
    );
  }

  Widget _buildBottomNavigationBar(BuildContext context) {
    final items = [
      BottomNavigationBarItem(icon: const Icon(Icons.credit_card), label: context.l10n.homeScreenBottomNavBarCardsCta),
      BottomNavigationBarItem(icon: const Icon(Icons.qr_code), label: context.l10n.homeScreenBottomNavBarQrCta),
      BottomNavigationBarItem(icon: const Icon(Icons.menu), label: context.l10n.homeScreenBottomNavBarMenuCta),
    ];

    final indicatorWidth = MediaQuery.of(context).size.width / items.length;
    const indicatorHeight = 2.0;
    const dividerHeight = 1.0;

    return BlocBuilder<HomeBloc, HomeState>(
      builder: (context, state) {
        return Stack(
          children: [
            BottomNavigationBar(
              currentIndex: state.tab.index,
              onTap: (value) {
                final homeTab = HomeTab.values[value];
                context.read<HomeBloc>().add(HomeTabPressed(homeTab));
              },
              items: items,
            ),
            Container(
              height: dividerHeight,
              width: double.infinity,
              color: context.colorScheme.outlineVariant,
            ),
            AnimatedPositioned(
              top: dividerHeight,
              height: indicatorHeight,
              width: indicatorWidth,
              left: indicatorWidth * state.tab.index,
              duration: kDefaultAnimationDuration,
              child: Container(color: context.colorScheme.primary),
            ),
          ],
        );
      },
    );
  }
}
