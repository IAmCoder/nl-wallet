import 'package:flutter/material.dart';

import '../../../../util/extension/build_context_extension.dart';
import '../../../../wallet_assets.dart';
import '../../../common/widget/button/confirm/confirm_buttons.dart';
import '../../../common/widget/button/primary_button.dart';
import '../../../common/widget/button/tertiary_button.dart';
import '../../../common/widget/sliver_sized_box.dart';

class WalletPersonalizeDigidErrorPage extends StatelessWidget {
  final VoidCallback onRetryPressed;
  final VoidCallback onHelpPressed;
  final String title, description;

  const WalletPersonalizeDigidErrorPage({
    required this.onRetryPressed,
    required this.onHelpPressed,
    required this.title,
    required this.description,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Column(
        children: [
          Expanded(
            child: Scrollbar(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: CustomScrollView(
                  slivers: [
                    const SliverSizedBox(height: 36),
                    SliverToBoxAdapter(
                      child: ExcludeSemantics(
                        child: Image.asset(
                          WalletAssets.illustration_digid_failure,
                          fit: context.isLandscape ? BoxFit.contain : BoxFit.fitWidth,
                          height: context.isLandscape ? 160 : null,
                          width: double.infinity,
                        ),
                      ),
                    ),
                    const SliverSizedBox(height: 24),
                    SliverToBoxAdapter(
                      child: MergeSemantics(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              title,
                              textAlign: TextAlign.start,
                              style: context.textTheme.displaySmall,
                            ),
                            const SizedBox(height: 8),
                            Text(
                              description,
                              textAlign: TextAlign.start,
                              style: context.textTheme.bodyLarge,
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SliverSizedBox(height: 32),
                  ],
                ),
              ),
            ),
          ),
          _buildBottomSection(context),
        ],
      ),
    );
  }

  Widget _buildBottomSection(BuildContext context) {
    return Column(
      children: [
        const Divider(height: 1),
        ConfirmButtons(
          primaryButton: PrimaryButton(
            text: Text(context.l10n.walletPersonalizeDigidErrorPageLoginWithDigidCta),
            onPressed: onRetryPressed,
            icon: Image.asset(WalletAssets.logo_digid),
          ),
          secondaryButton: TertiaryButton(
            onPressed: onHelpPressed,
            text: Text(context.l10n.walletPersonalizeDigidErrorPageNoDigidCta),
            icon: const Icon(Icons.help_outline_rounded),
          ),
        ),
      ],
    );
  }
}
