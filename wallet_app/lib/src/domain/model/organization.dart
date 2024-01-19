import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

import 'app_image_data.dart';
import 'attribute/converter/app_image_data_converter.dart';
import 'attribute/converter/localized_string_converter.dart';
import 'localized_text.dart';

part 'organization.g.dart';

@JsonSerializable(converters: [LocalizedStringConverter(), AppImageDataConverter()], explicitToJson: true)
class Organization extends Equatable {
  final String id;
  final LocalizedText legalName;
  final LocalizedText displayName;
  final LocalizedText? category;
  final LocalizedText? description;
  final AppImageData logo;
  final String? webUrl;
  final String? privacyPolicyUrl;
  final String? countryCode;
  final LocalizedText? city;
  final LocalizedText? department;
  final String? kvk;

  const Organization({
    required this.id,
    required this.legalName,
    required this.displayName,
    required this.category,
    required this.description,
    required this.logo,
    this.webUrl,
    this.privacyPolicyUrl,
    this.countryCode,
    this.city,
    this.department,
    this.kvk,
  });

  factory Organization.fromJson(Map<String, dynamic> json) => _$OrganizationFromJson(json);

  Map<String, dynamic> toJson() => _$OrganizationToJson(this);

  @override
  List<Object?> get props => [
        id,
        legalName,
        displayName,
        category,
        description,
        logo,
        webUrl,
        privacyPolicyUrl,
        city,
        countryCode,
        department,
        kvk,
      ];
}
