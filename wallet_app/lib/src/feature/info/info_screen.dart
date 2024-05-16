import 'package:flutter/material.dart';

import '../../navigation/secured_page_route.dart';
import '../../util/extension/build_context_extension.dart';
import '../common/widget/button/bottom_back_button.dart';
import '../common/widget/sliver_wallet_app_bar.dart';
import '../common/widget/text/body_text.dart';

/// Simple screen that renders the provided [title] and [description].
class InfoScreen extends StatelessWidget {
  final String title;
  final String description;

  const InfoScreen({
    required this.title,
    required this.description,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: CustomScrollView(
          slivers: [
            SliverWalletAppBar(title: title),
            SliverPadding(
              sliver: SliverToBoxAdapter(child: BodyText(description)),
              padding: const EdgeInsets.symmetric(horizontal: 16),
            ),
            const SliverFillRemaining(
              hasScrollBody: false,
              fillOverscroll: true,
              child: BottomBackButton(),
            ),
          ],
        ),
      ),
    );
  }

  static void show(
    BuildContext context, {
    bool secured = true,
    required String title,
    required String description,
  }) {
    Navigator.push(
      context,
      secured
          ? SecuredPageRoute(
              builder: (c) => InfoScreen(
                title: title,
                description: description,
              ),
            )
          : MaterialPageRoute(
              builder: (c) => InfoScreen(
                title: title,
                description: description,
              ),
            ),
    );
  }

  static void showDetailsIncorrect(BuildContext context) {
    show(
      context,
      title: context.l10n.detailsIncorrectScreenTitle,
      description: context.l10n.detailsIncorrectScreenDescription,
    );
  }
}
